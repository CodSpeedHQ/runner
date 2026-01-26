use crate::prelude::*;
use crate::{AllocatorLib, ebpf::MemtrackBpf};
use runner_shared::artifacts::MemtrackEvent as Event;
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

        let mut bpf = MemtrackBpf::new()?;
        bpf.attach_tracepoints()?;

        // Find and attach to all allocators
        let allocators = AllocatorLib::find_all()?;
        debug!("Found {} allocator instance(s)", allocators.len());

        for allocator in &allocators {
            debug!("Attaching uprobes to: {}", allocator.path.display());
            bpf.attach_allocator_probes(allocator.kind, &allocator.path)?;
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
