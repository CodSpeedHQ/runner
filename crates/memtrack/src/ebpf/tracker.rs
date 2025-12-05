use crate::ebpf::{Event, MemtrackBpf};
use anyhow::Result;
use log::debug;
use std::sync::mpsc::{self, Receiver};

pub struct Tracker {
    bpf: MemtrackBpf,
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

        let mut bpf = MemtrackBpf::new()?;
        bpf.attach_tracepoints()?;

        for libc_path in &libc_paths {
            debug!("Attaching uprobes to: {}", libc_path.display());
            bpf.attach_probes(libc_path)?;
        }

        Ok(Self { bpf })
    }

    /// Start tracking allocations for a specific PID
    ///
    /// Returns a receiver channel that will receive allocation events.
    /// The receiver will continue to produce events until the tracker is dropped.
    pub fn track(&mut self, pid: i32) -> Result<Receiver<Event>> {
        // Add the PID to track
        self.bpf.add_tracked_pid(pid)?;
        debug!("Tracking PID {pid}");

        // Start polling with channel
        let (_poller, event_rx) = self.bpf.start_polling_with_channel(10)?;

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

    /// Enable event tracking in the BPF program
    pub fn enable(&mut self) -> anyhow::Result<()> {
        self.bpf.enable_tracking()
    }

    /// Disable event tracking in the BPF program
    pub fn disable(&mut self) -> anyhow::Result<()> {
        self.bpf.disable_tracking()
    }
}
