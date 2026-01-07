use anyhow::Context;
use anyhow::Result;
use libbpf_rs::Link;
use libbpf_rs::skel::OpenSkel;
use libbpf_rs::skel::SkelBuilder;
use libbpf_rs::{MapCore, UprobeOpts};
use log::{debug, warn};
use paste::paste;
use std::mem::MaybeUninit;
use std::path::Path;

use crate::allocators::AllocatorKind;
use crate::ebpf::poller::RingBufferPoller;

pub mod memtrack_skel {
    include!(concat!(env!("OUT_DIR"), "/memtrack.skel.rs"));
}
pub use memtrack_skel::*;

/// Macro to attach a function with both entry and return probes.
/// Also generates a `try_attach_*` variant that logs errors instead of returning them.
macro_rules! attach_uprobe_uretprobe {
    ($name:ident, $prog_entry:ident, $prog_return:ident, $func_str:expr) => {
        fn $name(&mut self, lib_path: &Path) -> Result<()> {
            let link = self
                .skel
                .progs
                .$prog_entry
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
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
                    lib_path.display()
                ))?;
            self.probes.push(link);

            let link = self
                .skel
                .progs
                .$prog_return
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
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
                    lib_path.display()
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

            #[allow(dead_code)]
            fn [<try_attach_ $name>](&mut self, lib_path: &Path) {
                if let Err(e) = self.[<attach_ $name>](lib_path) {
                    debug!("{} not found in {}: {}", stringify!($name), lib_path.display(), e);
                }
            }
        }
    };
}

/// Macro to attach a function with only an entry probe (no return probe).
/// Also generates a `try_attach_*` variant that logs errors instead of returning them.
macro_rules! attach_uprobe {
    ($name:ident, $prog:ident, $func_str:expr) => {
        fn $name(&mut self, lib_path: &Path) -> Result<()> {
            let link = self
                .skel
                .progs
                .$prog
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
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
                    lib_path.display()
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

            #[allow(dead_code)]
            fn [<try_attach_ $name>](&mut self, lib_path: &Path) {
                if let Err(e) = self.[<attach_ $name>](lib_path) {
                    debug!("{} not found in {}: {}", stringify!($name), lib_path.display(), e);
                }
            }
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

    // =========================================================================
    // Standard libc allocation functions
    // =========================================================================
    attach_uprobe_uretprobe!(malloc);
    attach_uprobe_uretprobe!(calloc);
    attach_uprobe_uretprobe!(realloc);
    attach_uprobe_uretprobe!(aligned_alloc);
    attach_uprobe_uretprobe!(memalign);
    attach_uprobe!(free);

    // =========================================================================
    // jemalloc prefixed API
    // =========================================================================
    attach_uprobe_uretprobe!(je_malloc);
    attach_uprobe_uretprobe!(je_calloc);
    attach_uprobe_uretprobe!(je_realloc);
    attach_uprobe_uretprobe!(je_aligned_alloc);
    attach_uprobe_uretprobe!(je_memalign);
    attach_uprobe!(je_free);

    // jemalloc extended API
    attach_uprobe_uretprobe!(mallocx);
    attach_uprobe_uretprobe!(rallocx);
    attach_uprobe!(dallocx);

    // =========================================================================
    // mimalloc prefixed API
    // =========================================================================
    attach_uprobe_uretprobe!(mi_malloc);
    attach_uprobe_uretprobe!(mi_calloc);
    attach_uprobe_uretprobe!(mi_realloc);
    attach_uprobe_uretprobe!(mi_aligned_alloc);
    attach_uprobe_uretprobe!(mi_memalign);
    attach_uprobe!(mi_free);

    // mimalloc zero-initialized and aligned variants
    attach_uprobe_uretprobe!(mi_zalloc);
    attach_uprobe_uretprobe!(mi_malloc_aligned);
    attach_uprobe_uretprobe!(mi_zalloc_aligned);
    attach_uprobe_uretprobe!(mi_realloc_aligned);

    // =========================================================================
    // Attach methods grouped by allocator
    // =========================================================================

    /// Attach standard allocation probes (libc-style: malloc, free, calloc, etc.)
    /// This works for libc and allocators that export standard symbol names.
    pub fn attach_standard_probes(&mut self, lib_path: &Path) -> Result<()> {
        self.attach_malloc(lib_path)?;
        self.attach_free(lib_path)?;
        self.attach_calloc(lib_path)?;
        self.attach_realloc(lib_path)?;
        self.attach_aligned_alloc(lib_path)?;
        self.try_attach_memalign(lib_path);
        Ok(())
    }

    /// Attach probes for a specific allocator kind.
    /// This attaches both standard probes (if the allocator exports them) and
    /// allocator-specific prefixed probes.
    pub fn attach_allocator_probes(&mut self, kind: AllocatorKind, lib_path: &Path) -> Result<()> {
        debug!(
            "Attaching {} probes to: {}",
            kind.name(),
            lib_path.display()
        );

        match kind {
            AllocatorKind::Libc => {
                // Libc only has standard probes, and they must succeed
                self.attach_standard_probes(lib_path)
            }
            AllocatorKind::Jemalloc => {
                // Try standard names (jemalloc may export these as drop-in replacements)
                let _ = self.attach_standard_probes(lib_path);
                self.attach_jemalloc_probes(lib_path)
            }
            AllocatorKind::Mimalloc => {
                // Try standard names (mimalloc may export these as drop-in replacements)
                let _ = self.attach_standard_probes(lib_path);
                self.attach_mimalloc_probes(lib_path)
            }
        }
    }

    /// Attach jemalloc-specific probes (prefixed and extended API).
    fn attach_jemalloc_probes(&mut self, lib_path: &Path) -> Result<()> {
        // Prefixed standard API
        self.try_attach_je_malloc(lib_path);
        self.try_attach_je_free(lib_path);
        self.try_attach_je_calloc(lib_path);
        self.try_attach_je_realloc(lib_path);
        self.try_attach_je_aligned_alloc(lib_path);
        self.try_attach_je_memalign(lib_path);

        // Extended API
        self.try_attach_mallocx(lib_path);
        self.try_attach_rallocx(lib_path);
        self.try_attach_dallocx(lib_path);

        Ok(())
    }

    /// Attach mimalloc-specific probes (mi_* API).
    fn attach_mimalloc_probes(&mut self, lib_path: &Path) -> Result<()> {
        // Core API
        self.try_attach_mi_malloc(lib_path);
        self.try_attach_mi_free(lib_path);
        self.try_attach_mi_calloc(lib_path);
        self.try_attach_mi_realloc(lib_path);
        self.try_attach_mi_aligned_alloc(lib_path);
        self.try_attach_mi_memalign(lib_path);

        // Zero-initialized and aligned variants
        self.try_attach_mi_zalloc(lib_path);
        self.try_attach_mi_malloc_aligned(lib_path);
        self.try_attach_mi_zalloc_aligned(lib_path);
        self.try_attach_mi_realloc_aligned(lib_path);

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
