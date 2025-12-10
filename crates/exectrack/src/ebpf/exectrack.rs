use anyhow::{Context, Result};
use codspeed_bpf::ProcessTracking;
use codspeed_bpf::{RingBufferPoller, attach_tracepoint};
use libbpf_rs::Link;
use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use std::mem::MaybeUninit;

pub mod exectrack_skel {
    include!(concat!(env!("OUT_DIR"), "/exectrack.skel.rs"));
}
pub use exectrack_skel::*;

pub struct ExectrackBpf {
    skel: Box<ExectrackSkel<'static>>,
    probes: Vec<Link>,
}

impl ExectrackBpf {
    pub fn new() -> Result<Self> {
        let builder = ExectrackSkelBuilder::default();
        let open_object = Box::leak(Box::new(MaybeUninit::uninit()));
        let open_skel = builder
            .open(open_object)
            .context("Failed to open exectrack BPF skeleton")?;

        let skel = Box::new(
            open_skel
                .load()
                .context("Failed to load exectrack BPF skeleton")?,
        );

        Ok(Self {
            skel,
            probes: Vec::new(),
        })
    }

    // Use the shared macro from codspeed-bpf for attaching tracepoints
    attach_tracepoint!(attach_sched_fork, tracepoint_sched_fork);
    attach_tracepoint!(attach_sched_exec, tracepoint_sched_exec);
    attach_tracepoint!(attach_sched_exit, tracepoint_sched_exit);

    pub fn attach_tracepoints(&mut self) -> Result<()> {
        self.attach_sched_fork()?;
        self.attach_sched_exec()?;
        self.attach_sched_exit()?;
        Ok(())
    }

    /// Start polling with an mpsc channel for events
    pub fn start_polling_with_channel(
        &self,
        poll_timeout_ms: u64,
    ) -> Result<(
        RingBufferPoller,
        std::sync::mpsc::Receiver<super::events::Event>,
    )> {
        RingBufferPoller::with_channel(&self.skel.maps.events, poll_timeout_ms)
    }
}

impl ProcessTracking for ExectrackBpf {
    fn tracked_pids_map(&self) -> &impl libbpf_rs::MapCore {
        &self.skel.maps.tracked_pids
    }
}
