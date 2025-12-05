use anyhow::Context;
use anyhow::Result;
use libbpf_rs::Link;
use libbpf_rs::skel::OpenSkel;
use libbpf_rs::skel::SkelBuilder;
use libbpf_rs::{MapCore, UprobeOpts};
use log::warn;
use paste::paste;
use std::mem::MaybeUninit;
use std::path::Path;

use crate::ebpf::poller::RingBufferPoller;

pub mod memtrack_skel {
    include!(concat!(env!("OUT_DIR"), "/memtrack.skel.rs"));
}
pub use memtrack_skel::*;

/// Macro to attach a function with both entry and return probes
macro_rules! attach_uprobe_uretprobe {
    ($name:ident, $prog_entry:ident, $prog_return:ident, $func_str:expr) => {
        fn $name(&mut self, libc_path: &Path) -> Result<()> {
            let link = self
                .skel
                .progs
                .$prog_entry
                .attach_uprobe_with_opts(
                    -1,
                    libc_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: false,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    $func_str,
                    libc_path.display()
                ))?;
            self.probes.push(link);

            let link = self
                .skel
                .progs
                .$prog_return
                .attach_uprobe_with_opts(
                    -1,
                    libc_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: true,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uretprobe in {}",
                    $func_str,
                    libc_path.display()
                ))?;
            self.probes.push(link);

            Ok(())
        }
    };
    ($name:ident) => {
        paste! {
            attach_uprobe_uretprobe!(
                [<attach_ $name>],
                [<uprobe_ $name>],
                [<uretprobe_ $name>],
                stringify!($name)
            );
        }
    };
}

macro_rules! attach_uprobe {
    ($name:ident, $prog:ident, $func_str:expr) => {
        fn $name(&mut self, libc_path: &Path) -> Result<()> {
            let link = self
                .skel
                .progs
                .$prog
                .attach_uprobe_with_opts(
                    -1,
                    libc_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: false,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    $func_str,
                    libc_path.display()
                ))?;
            self.probes.push(link);
            Ok(())
        }
    };
    ($name:ident) => {
        paste! {
            attach_uprobe!(
                [<attach_ $name>],
                [<uprobe_ $name>],
                stringify!($name)
            );
        }
    };
}

macro_rules! attach_tracepoint {
    ($func:ident, $prog:ident) => {
        fn $func(&mut self) -> Result<()> {
            let link = self
                .skel
                .progs
                .$prog
                .attach()
                .context(format!("Failed to attach {} tracepoint", stringify!($prog)))?;
            self.probes.push(link);
            Ok(())
        }
    };
    ($name:ident) => {
        paste! {
            attach_tracepoint!([<attach_ $name>], [<tracepoint_ $name>]);
        }
    };
}

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

impl Drop for MemtrackBpf {
    fn drop(&mut self) {
        if self.probes.len() > 10 {
            warn!(
                "Dropping the MemtrackBpf instance, this can take some time when having many probes attached"
            );
        }
    }
}
