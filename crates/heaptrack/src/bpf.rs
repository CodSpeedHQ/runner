use anyhow::Context;
use anyhow::Result;
use libbpf_rs::Link;
use libbpf_rs::skel::OpenSkel;
use libbpf_rs::skel::SkelBuilder;
use libbpf_rs::{MapCore, UprobeOpts};
use log::warn;
use std::mem::MaybeUninit;
use std::path::Path;

use crate::poller::RingBufferPoller;

pub mod heaptrack_skel {
    include!(concat!(env!("OUT_DIR"), "/heaptrack.skel.rs"));
}
pub use heaptrack_skel::*;

pub struct HeaptrackBpf {
    skel: Box<HeaptrackSkel<'static>>,
    probes: Vec<Link>,
}

impl HeaptrackBpf {
    pub fn new() -> Result<Self> {
        // Build and open the syscalls BPF program
        let builder = HeaptrackSkelBuilder::default();
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

    pub fn add_tracked_pid(&mut self, pid: i32) -> Result<()> {
        self.skel
            .maps
            .tracked_pids
            .update(
                &pid.to_le_bytes(),
                &1u8.to_le_bytes(),
                libbpf_rs::MapFlags::ANY,
            )
            .context("Failed to add PID to uprobes tracked set")?;

        Ok(())
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

    pub fn attach_malloc(&mut self, libc_path: &Path) -> Result<()> {
        let malloc_opts = UprobeOpts {
            func_name: Some("malloc".to_string()),
            retprobe: false,
            ..Default::default()
        };

        let link = self
            .skel
            .progs
            .uprobe_malloc
            .attach_uprobe_with_opts(-1, libc_path, 0, malloc_opts)
            .context(format!(
                "Failed to attach malloc uprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        // Attach malloc return
        let malloc_ret_opts = UprobeOpts {
            func_name: Some("malloc".to_string()),
            retprobe: true,
            ..Default::default()
        };
        let link = self
            .skel
            .progs
            .uretprobe_malloc
            .attach_uprobe_with_opts(-1, libc_path, 0, malloc_ret_opts)
            .context(format!(
                "Failed to attach malloc uretprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        Ok(())
    }

    pub fn attach_free(&mut self, libc_path: &Path) -> Result<()> {
        let free_opts = UprobeOpts {
            func_name: Some("free".to_string()),
            retprobe: false,
            ..Default::default()
        };

        let link = self
            .skel
            .progs
            .uprobe_free
            .attach_uprobe_with_opts(-1, libc_path, 0, free_opts)
            .context(format!(
                "Failed to attach free uprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        Ok(())
    }

    pub fn attach_calloc(&mut self, libc_path: &Path) -> Result<()> {
        let calloc_opts = UprobeOpts {
            func_name: Some("calloc".to_string()),
            retprobe: false,
            ..Default::default()
        };

        let link = self
            .skel
            .progs
            .uprobe_calloc
            .attach_uprobe_with_opts(-1, libc_path, 0, calloc_opts)
            .context(format!(
                "Failed to attach calloc uprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        // Attach calloc return
        let calloc_ret_opts = UprobeOpts {
            func_name: Some("calloc".to_string()),
            retprobe: true,
            ..Default::default()
        };
        let link = self
            .skel
            .progs
            .uretprobe_calloc
            .attach_uprobe_with_opts(-1, libc_path, 0, calloc_ret_opts)
            .context(format!(
                "Failed to attach calloc uretprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        Ok(())
    }

    pub fn attach_realloc(&mut self, libc_path: &Path) -> Result<()> {
        let realloc_opts = UprobeOpts {
            func_name: Some("realloc".to_string()),
            retprobe: false,
            ..Default::default()
        };

        let link = self
            .skel
            .progs
            .uprobe_realloc
            .attach_uprobe_with_opts(-1, libc_path, 0, realloc_opts)
            .context(format!(
                "Failed to attach realloc uprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        // Attach realloc return
        let realloc_ret_opts = UprobeOpts {
            func_name: Some("realloc".to_string()),
            retprobe: true,
            ..Default::default()
        };
        let link = self
            .skel
            .progs
            .uretprobe_realloc
            .attach_uprobe_with_opts(-1, libc_path, 0, realloc_ret_opts)
            .context(format!(
                "Failed to attach realloc uretprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        Ok(())
    }

    pub fn attach_aligned_alloc(&mut self, libc_path: &Path) -> Result<()> {
        let aligned_alloc_opts = UprobeOpts {
            func_name: Some("aligned_alloc".to_string()),
            retprobe: false,
            ..Default::default()
        };

        let link = self
            .skel
            .progs
            .uprobe_aligned_alloc
            .attach_uprobe_with_opts(-1, libc_path, 0, aligned_alloc_opts)
            .context(format!(
                "Failed to attach aligned_alloc uprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        // Attach aligned_alloc return
        let aligned_alloc_ret_opts = UprobeOpts {
            func_name: Some("aligned_alloc".to_string()),
            retprobe: true,
            ..Default::default()
        };
        let link = self
            .skel
            .progs
            .uretprobe_aligned_alloc
            .attach_uprobe_with_opts(-1, libc_path, 0, aligned_alloc_ret_opts)
            .context(format!(
                "Failed to attach aligned_alloc uretprobe in {}",
                libc_path.display()
            ))?;
        self.probes.push(link);

        Ok(())
    }

    pub fn attach_tracepoints(&mut self) -> Result<()> {
        // Attach sched_process_fork tracepoint to track child processes (from uprobes)
        let link = self
            .skel
            .progs
            .tracepoint_sched_fork
            .attach()
            .context("Failed to attach sched_process_fork tracepoint (uprobes)")?;
        self.probes.push(link);

        // Attach sys_enter_execve tracepoint
        let link = self
            .skel
            .progs
            .tracepoint_sys_execve
            .attach()
            .context("Failed to attach sys_enter_execve tracepoint")?;
        self.probes.push(link);

        Ok(())
    }

    /// Start polling with an mpsc channel for events
    pub fn start_polling_with_channel(
        &self,
        poll_timeout_ms: u64,
    ) -> Result<(
        RingBufferPoller,
        std::sync::mpsc::Receiver<crate::events::Event>,
    )> {
        // Use the syscalls skeleton's ring buffer (both programs share the same one)
        RingBufferPoller::with_channel(&self.skel.maps.events, poll_timeout_ms)
    }
}

impl Drop for HeaptrackBpf {
    fn drop(&mut self) {
        if self.probes.len() > 10 {
            warn!(
                "Dropping the HeaptrackEbpf instance, this can take some time when having many probes attached"
            );
        }
    }
}
