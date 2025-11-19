use anyhow::Result;
use log::debug;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::bpf::HeaptrackBpf;
use crate::events::Event;

/// High-level tracker for monitoring memory allocations
///
/// This provides a simple API for tracking allocations in a process by PID.
///
/// # Example
///
/// ```ignore
/// use heaptrack::Tracker;
/// use std::process::Command;
///
/// // Spawn your process
/// let child = Command::new("./my_program").spawn()?;
/// let pid = child.id() as i32;
///
/// // Start tracking
/// let tracker = Tracker::new()?;
/// let events = tracker.track(pid)?;
///
/// // Process events
/// for event in events {
///     println!("Event: {:?}", event);
/// }
/// ```
pub struct Tracker {
    heaptrack: HeaptrackBpf,
    stopped: AtomicBool,
}

impl Tracker {
    /// Create a new tracker instance
    ///
    /// This will:
    /// - Initialize the BPF subsystem
    /// - Bump memlock limits
    /// - Attach uprobes to all libc instances
    /// - Attach tracepoints for fork tracking
    pub fn new() -> Result<Self> {
        // Bump memlock limits
        Self::bump_memlock_rlimit()?;

        // Find and attach to all libc instances
        let libc_paths = crate::libc::find_libc_paths()?;
        debug!("Found {} libc instance(s)", libc_paths.len());

        let mut heaptrack = HeaptrackBpf::new()?;
        heaptrack.attach_tracepoints()?;

        for libc_path in &libc_paths {
            debug!("Attaching uprobes to: {}", libc_path.display());
            heaptrack.attach_malloc(libc_path)?;
            heaptrack.attach_free(libc_path)?;
            heaptrack.attach_calloc(libc_path)?;
            heaptrack.attach_realloc(libc_path)?;
            heaptrack.attach_aligned_alloc(libc_path)?;
        }

        Ok(Self {
            heaptrack,
            stopped: AtomicBool::new(false),
        })
    }

    /// Start tracking allocations for a specific PID
    ///
    /// Returns a receiver channel that will receive allocation events.
    /// The receiver will continue to produce events until the tracker is dropped.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to track
    ///
    /// # Returns
    ///
    /// A `Receiver<Event>` that yields allocation events
    pub fn track(&mut self, pid: i32) -> Result<Receiver<Event>> {
        // Add the PID to track
        self.heaptrack.add_tracked_pid(pid)?;
        debug!("Tracking PID {pid}");

        // Start polling with channel
        let (_poller, event_rx) = self.heaptrack.start_polling_with_channel(10)?;

        // Keep the poller alive by moving it into the channel
        // When the receiver is dropped, the poller will also be dropped
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            // Keep poller alive
            let _p = _poller;
            while let Ok(event) = event_rx.recv() {
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// Track multiple PIDs simultaneously
    ///
    /// Returns a receiver channel that will receive allocation events from all tracked PIDs.
    ///
    /// # Arguments
    ///
    /// * `pids` - A slice of process IDs to track
    ///
    /// # Returns
    ///
    /// A `Receiver<Event>` that yields allocation events from all tracked processes
    pub fn track_multiple(mut self, pids: &[i32]) -> Result<Receiver<Event>> {
        // Add all PIDs to track
        for &pid in pids {
            self.heaptrack.add_tracked_pid(pid)?;
            debug!("Tracking PID {pid}");
        }

        // Start polling with channel
        let (_poller, event_rx) = self.heaptrack.start_polling_with_channel(10)?;

        // Keep the poller alive by moving it into the channel
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _p = _poller;

            loop {
                if self.stopped.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                let Ok(event) = event_rx.try_recv() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    fn bump_memlock_rlimit() -> Result<()> {
        let rlimit = libc::rlimit {
            rlim_cur: libc::RLIM_INFINITY,
            rlim_max: libc::RLIM_INFINITY,
        };

        let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) };
        if ret != 0 {
            anyhow::bail!("Failed to increase rlimit");
        }

        Ok(())
    }

    pub fn stop_threads(&mut self) -> anyhow::Result<()> {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
