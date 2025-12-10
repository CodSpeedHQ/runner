use crate::ebpf::{Event, ExectrackBpf};
use anyhow::Result;
use codspeed_bpf::{ProcessTracking, RingBufferPoller, bump_memlock_rlimit};
use log::debug;
use std::sync::mpsc::{self, Receiver};

pub struct Tracker {
    bpf: ExectrackBpf,
    poller: Option<RingBufferPoller>,
}

impl Tracker {
    /// Create a new tracker instance
    ///
    /// This will:
    /// - Initialize the BPF subsystem
    /// - Bump memlock limits
    /// - Attach tracepoints for process tracking
    pub fn new() -> Result<Self> {
        // Bump memlock limits using shared utility
        bump_memlock_rlimit()?;

        let mut bpf = ExectrackBpf::new()?;
        bpf.attach_tracepoints()?;

        Ok(Self { bpf, poller: None })
    }

    /// Start tracking execution events for a specific PID
    ///
    /// Returns a receiver channel that will receive execution events.
    /// The receiver will continue to produce events until the tracker is dropped.
    pub fn track(&mut self, pid: i32) -> Result<Receiver<Event>> {
        // Add the PID to track
        self.bpf.add_tracked_pid(pid)?;
        debug!("Tracking PID {pid} for execution events");

        // Start polling with channel
        let (poller, event_rx) = self.bpf.start_polling_with_channel(10)?;
        self.poller = Some(poller);

        // Forward events from the poller channel to a new channel
        // This allows the caller to drop the tracker and still receive events
        // until the original channel closes
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
