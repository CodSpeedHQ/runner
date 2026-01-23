use crate::prelude::*;
use libbpf_rs::Link;
use libbpf_rs::skel::OpenSkel;
use libbpf_rs::skel::SkelBuilder;
use libbpf_rs::{MapCore, UprobeOpts};
use paste::paste;
use std::mem::MaybeUninit;
use std::path::Path;

use crate::allocators::AllocatorKind;
use crate::ebpf::poller::RingBufferPoller;

pub mod memtrack_skel {
    include!(concat!(env!("OUT_DIR"), "/memtrack.skel.rs"));
}
pub use memtrack_skel::*;

/// Resolve symbol offset from .symtab to ensure that libbpf can find it. Otherwise
/// it will print a warning at runtime.
fn ensure_symbol_exists(lib_path: &Path, symbol_name: &str) -> Result<()> {
    use object::{Object, ObjectSymbol};

    let data = std::fs::read(lib_path)?;
    let file = object::File::parse(&*data)?;

    // Check both regular and dynamic symbols
    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        if !symbol.is_definition() {
            continue;
        }

        let Ok(name) = symbol.name() else {
            continue;
        };

        if name == symbol_name {
            let addr = symbol.address();
            if addr != 0 {
                return Ok(());
            }
        }
    }

    bail!("Symbol {symbol_name} not found in {}", lib_path.display())
}

/// Macro to attach a function with both entry and return probes.
/// Also generates a `try_attach_*` variant that logs errors instead of returning them.
///
/// Uses offset-based attachment by resolving symbols from .symtab.
/// Fails if the symbol is not found.
macro_rules! attach_uprobe_uretprobe {
    ($name:ident, $prog_entry:ident, $prog_return:ident) => {
        fn $name(&mut self, lib_path: &Path, symbol: &str) -> Result<()> {
            ensure_symbol_exists(lib_path, symbol)?;

            // Attach entry probe at function entry via func_name
            let link = self
                .skel
                .progs
                .$prog_entry
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
                    0,
                    UprobeOpts {
                        retprobe: false,
                        func_name: Some(symbol.to_owned()),
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    symbol,
                    lib_path.display()
                ))?;
            self.probes.push(link);

            // Attach return probe at function entry via func_name
            let link = self
                .skel
                .progs
                .$prog_return
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
                    0,
                    UprobeOpts {
                        retprobe: true,
                        func_name: Some(symbol.to_owned()),
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uretprobe in {}",
                    symbol,
                    lib_path.display()
                ))?;
            self.probes.push(link);

            Ok(())
        }

        paste! {
            fn [<try_ $name>](&mut self, lib_path: &Path, symbol: &str) {
                let result = self.$name(lib_path, symbol);
                log::trace!("{} uprobe attach result: {:?}", symbol, result);
            }
        }
    };
}

/// Macro to attach a function with only an entry probe (no return probe).
/// Also generates a `try_attach_*` variant that logs errors instead of returning them.
///
/// Uses offset-based attachment by resolving symbols from .symtab.
/// Fails if the symbol is not found.
macro_rules! attach_uprobe {
    ($name:ident, $prog:ident) => {
        fn $name(&mut self, lib_path: &Path, symbol: &str) -> Result<()> {
            ensure_symbol_exists(lib_path, symbol)?;

            let link = self
                .skel
                .progs
                .$prog
                .attach_uprobe_with_opts(
                    -1,
                    lib_path,
                    0,
                    UprobeOpts {
                        retprobe: false,
                        func_name: Some(symbol.to_owned()),
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    symbol,
                    lib_path.display()
                ))?;
            self.probes.push(link);
            Ok(())
        }

        paste! {
            fn [<try_ $name>](&mut self, lib_path: &Path, symbol: &str) {
                let result = self.$name(lib_path, symbol);
                log::trace!("{} uprobe attach result: {:?}", symbol, result);
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
    // Allocation probe functions (symbol passed at call time)
    // =========================================================================
    attach_uprobe_uretprobe!(attach_malloc, uprobe_malloc, uretprobe_malloc);
    attach_uprobe_uretprobe!(attach_calloc, uprobe_calloc, uretprobe_calloc);
    attach_uprobe_uretprobe!(attach_realloc, uprobe_realloc, uretprobe_realloc);
    attach_uprobe_uretprobe!(
        attach_aligned_alloc,
        uprobe_aligned_alloc,
        uretprobe_aligned_alloc
    );
    attach_uprobe_uretprobe!(attach_memalign, uprobe_memalign, uretprobe_memalign);
    attach_uprobe!(attach_free, uprobe_free);

    // =========================================================================
    // Attach methods grouped by allocator
    // =========================================================================

    /// Attach standard library allocation probes (libc-style: malloc, free, calloc, etc.)
    /// This works for libc and allocators that export standard symbol names.
    /// For non-libc allocators, standard names are optional - just try them silently.
    pub fn attach_libc_probes(&mut self, lib_path: &Path) -> Result<()> {
        self.try_attach_malloc(lib_path, "malloc");
        self.try_attach_calloc(lib_path, "calloc");
        self.try_attach_realloc(lib_path, "realloc");
        self.try_attach_free(lib_path, "free");
        self.try_attach_aligned_alloc(lib_path, "aligned_alloc");
        self.try_attach_memalign(lib_path, "posix_memalign");
        self.try_attach_memalign(lib_path, "memalign");
        Ok(())
    }

    /// Attach C++ operator new/delete probes.
    /// These are mangled C++ symbols that wrap the underlying allocator.
    /// C++ operators have identical signatures to malloc/free, so we reuse those handlers.
    pub fn attach_libcpp_probes(&mut self, lib_path: &Path) -> Result<()> {
        self.try_attach_malloc(lib_path, "_Znwm"); // operator new(size_t)
        self.try_attach_malloc(lib_path, "_Znam"); // operator new[](size_t)
        self.try_attach_malloc(lib_path, "_ZnwmSt11align_val_t"); // operator new(size_t, std::align_val_t)
        self.try_attach_malloc(lib_path, "_ZnamSt11align_val_t"); // operator new[](size_t, std::align_val_t)
        self.try_attach_free(lib_path, "_ZdlPv"); // operator delete(void*)
        self.try_attach_free(lib_path, "_ZdaPv"); // operator delete[](void*)
        self.try_attach_free(lib_path, "_ZdlPvm"); // operator delete(void*, size_t) - C++14 sized delete
        self.try_attach_free(lib_path, "_ZdaPvm"); // operator delete[](void*, size_t) - C++14 sized delete
        self.try_attach_free(lib_path, "_ZdlPvSt11align_val_t"); // operator delete(void*, std::align_val_t)
        self.try_attach_free(lib_path, "_ZdaPvSt11align_val_t"); // operator delete[](void*, std::align_val_t)
        self.try_attach_free(lib_path, "_ZdlPvmSt11align_val_t"); // operator delete(void*, size_t, std::align_val_t)
        self.try_attach_free(lib_path, "_ZdaPvmSt11align_val_t"); // operator delete[](void*, size_t, std::align_val_t)

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
                self.attach_libc_probes(lib_path)
            }
            AllocatorKind::LibCpp => {
                // libc++ exports C++ operator new/delete symbols
                self.attach_libcpp_probes(lib_path)
            }
            AllocatorKind::Jemalloc => {
                // Try standard names (jemalloc may export these as drop-in replacements)
                let _ = self.attach_libc_probes(lib_path);
                // Try C++ operators (jemalloc exports these for C++ programs)
                let _ = self.attach_libcpp_probes(lib_path);
                self.attach_jemalloc_probes(lib_path)
            }
            AllocatorKind::Mimalloc => {
                // Try standard names (mimalloc may export these as drop-in replacements)
                let _ = self.attach_libc_probes(lib_path);
                // Try C++ operators (mimalloc exports these for C++ programs)
                let _ = self.attach_libcpp_probes(lib_path);
                self.attach_mimalloc_probes(lib_path)
            }
        }
    }

    /// Attach jemalloc-specific probes (prefixed and extended API).
    fn attach_jemalloc_probes(&mut self, lib_path: &Path) -> Result<()> {
        // The following functions are used in Rust when setting a global allocator:
        // - rust_alloc: _rjem_malloc and _rjem_mallocx
        // - rust_alloc_zeroed: _rjem_mallocx / _rjem_calloc
        // - rust_dealloc: _rjem_sdallocx
        // - rust_realloc: _rjem_realloc / _rjem_rallocx

        // Prefixed standard API
        self.try_attach_malloc(lib_path, "_rjem_malloc");
        self.try_attach_malloc(lib_path, "_rjem_mallocx"); // Also used for `calloc`
        self.try_attach_calloc(lib_path, "_rjem_calloc");
        self.try_attach_realloc(lib_path, "_rjem_realloc");
        self.try_attach_realloc(lib_path, "_rjem_rallocx");
        self.try_attach_aligned_alloc(lib_path, "_rjem_aligned_alloc");
        self.try_attach_memalign(lib_path, "_rjem_memalign");
        self.try_attach_free(lib_path, "_rjem_free");
        self.try_attach_free(lib_path, "_rjem_sdallocx");

        Ok(())
    }

    /// Attach mimalloc-specific probes (mi_* API).
    fn attach_mimalloc_probes(&mut self, lib_path: &Path) -> Result<()> {
        // The following functions are used in Rust when setting a global allocator:
        // - mi_malloc_aligned
        // - mi_free
        // - mi_realloc_aligned
        // - mi_zalloc_aligned

        // Core API
        self.try_attach_malloc(lib_path, "mi_malloc");
        self.try_attach_malloc(lib_path, "mi_malloc_aligned");
        self.try_attach_calloc(lib_path, "mi_calloc");
        self.try_attach_realloc(lib_path, "mi_realloc");
        self.try_attach_aligned_alloc(lib_path, "mi_aligned_alloc");
        self.try_attach_memalign(lib_path, "mi_memalign");
        self.try_attach_free(lib_path, "mi_free");

        // Zero-initialized and aligned variants
        self.try_attach_calloc(lib_path, "mi_zalloc");
        self.try_attach_calloc(lib_path, "mi_zalloc_aligned");
        self.try_attach_realloc(lib_path, "mi_realloc_aligned");

        Ok(())
    }
    attach_tracepoint!(sched_fork);

    pub fn attach_tracepoints(&mut self) -> Result<()> {
        self.attach_sched_fork()?;
        Ok(())
    }

    /// Start polling with an mpsc channel for events
    pub fn start_polling_with_channel(
        &self,
        poll_timeout_ms: u64,
    ) -> Result<(
        RingBufferPoller,
        std::sync::mpsc::Receiver<runner_shared::artifacts::MemtrackEvent>,
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
