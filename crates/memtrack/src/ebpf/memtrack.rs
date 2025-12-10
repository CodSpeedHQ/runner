use anyhow::Context;
use anyhow::Result;
use codspeed_bpf::ProcessTracking;
use codspeed_bpf::RingBufferPoller;
use libbpf_rs::Link;
use libbpf_rs::MapCore;
use libbpf_rs::skel::OpenSkel;
use libbpf_rs::skel::SkelBuilder;
use log::warn;
use std::mem::MaybeUninit;
use std::path::Path;

// Use shared macros from codspeed-bpf
use codspeed_bpf::{attach_tracepoint, attach_uprobe, attach_uprobe_uretprobe};

pub mod memtrack_skel {
    include!(concat!(env!("OUT_DIR"), "/memtrack.skel.rs"));
}
pub use memtrack_skel::*;

pub struct MemtrackBpf {
    skel: Box<MemtrackSkel<'static>>,
    probes: Vec<Link>,
}

impl MemtrackBpf {
    pub fn new() -> Result<Self> {
        // Build and open the syscalls BPF program
        let builder = MemtrackSkelBuilder::default();
        let open_object = Box::leak(Box::new(MaybeUninit::uninit()));
        let open_skel = builder
            .open(open_object)
            .context("Failed to open syscalls BPF skeleton")?;

        let skel = Box::new(
            open_skel
                .load()
                .context("Failed to load syscalls BPF skeleton")?,
        );

        Ok(Self {
            skel,
            probes: Vec::new(),
        })
    }

    /// Enable event tracking
    pub fn enable_tracking(&mut self) -> Result<()> {
        let key = 0u32;
        let value = true as u8;
        self.skel
            .maps
            .tracking_enabled
            .update(
                &key.to_le_bytes(),
                &value.to_le_bytes(),
                libbpf_rs::MapFlags::ANY,
            )
            .context("Failed to enable tracking")?;
        Ok(())
    }

    /// Disable event tracking
    pub fn disable_tracking(&mut self) -> Result<()> {
        let key = 0u32;
        let value = false as u8;
        self.skel
            .maps
            .tracking_enabled
            .update(
                &key.to_le_bytes(),
                &value.to_le_bytes(),
                libbpf_rs::MapFlags::ANY,
            )
            .context("Failed to disable tracking")?;
        Ok(())
    }

    attach_uprobe_uretprobe!(malloc);
    attach_uprobe_uretprobe!(calloc);
    attach_uprobe_uretprobe!(realloc);
    attach_uprobe_uretprobe!(aligned_alloc);
    attach_uprobe!(free);

    pub fn attach_probes(&mut self, libc_path: &Path) -> Result<()> {
        self.attach_malloc(libc_path)?;
        self.attach_free(libc_path)?;
        self.attach_calloc(libc_path)?;
        self.attach_realloc(libc_path)?;
        self.attach_aligned_alloc(libc_path)?;
        Ok(())
    }

    attach_tracepoint!(sched_fork);
    attach_tracepoint!(sys_execve);

    pub fn attach_tracepoints(&mut self) -> Result<()> {
        self.attach_sched_fork()?;
        self.attach_sys_execve()?;
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
        // Use the syscalls skeleton's ring buffer (both programs share the same one)
        RingBufferPoller::with_channel(&self.skel.maps.events, poll_timeout_ms)
    }
}

impl ProcessTracking for MemtrackBpf {
    fn tracked_pids_map(&self) -> &impl MapCore {
        &self.skel.maps.tracked_pids
    }
}

impl Drop for MemtrackBpf {
    fn drop(&mut self) {
        if self.probes.len() > 10 {
            warn!(
                "Dropping the MemtrackBpf instance, this can take some time when having many probes attached"
            );
        }
    }
}
