//! Generic allocator discovery infrastructure.
//!
//! This module provides a framework for discovering and attaching to different
//! memory allocators. It's designed to be easily extensible for adding new allocators.

use std::path::PathBuf;

mod dynamic;
mod static_linked;

/// Represents the different allocator types we support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocatorKind {
    /// Standard C library (glibc, musl, etc.)
    Libc,
    /// C++ standard library (libstdc++, libc++) - provides operator new/delete
    LibCpp,
    /// jemalloc - used by FreeBSD, Firefox, many Rust projects
    Jemalloc,
    /// mimalloc - Microsoft's allocator
    Mimalloc,
    // Future allocators:
    // Tcmalloc,
    // Hoard,
    // Rpmalloc,
}

impl AllocatorKind {
    /// Returns all supported allocator kinds.
    pub fn all() -> &'static [AllocatorKind] {
        &[
            AllocatorKind::Libc,
            AllocatorKind::LibCpp,
            AllocatorKind::Jemalloc,
            AllocatorKind::Mimalloc,
        ]
    }

    /// Returns a human-readable name for the allocator.
    pub fn name(&self) -> &'static str {
        match self {
            AllocatorKind::Libc => "libc",
            AllocatorKind::LibCpp => "libc++",
            AllocatorKind::Jemalloc => "jemalloc",
            AllocatorKind::Mimalloc => "mimalloc",
        }
    }

    /// Returns true if this allocator is required (must be found).
    pub fn is_required(&self) -> bool {
        matches!(self, AllocatorKind::Libc)
    }

    /// Returns the symbol names used to detect this allocator in binaries.
    pub fn symbols(&self) -> &'static [&'static str] {
        match self {
            AllocatorKind::Libc => &["malloc", "free"],
            AllocatorKind::LibCpp => &["_Znwm", "_Znam", "_ZdlPv", "_ZdaPv"],
            AllocatorKind::Jemalloc => &["_rjem_malloc", "_rjem_free"],
            AllocatorKind::Mimalloc => &["mi_malloc_aligned", "mi_malloc", "mi_free"],
        }
    }
}

/// Discovered allocator library with its kind and path.
#[derive(Debug, Clone)]
pub struct AllocatorLib {
    pub kind: AllocatorKind,
    pub path: PathBuf,
}

impl AllocatorLib {
    pub fn find_all() -> anyhow::Result<Vec<AllocatorLib>> {
        let mut allocators = static_linked::find_all()?;
        allocators.extend(dynamic::find_all()?);
        Ok(allocators)
    }
}

/// Check if a file is an ELF binary by reading its magic bytes.
fn is_elf(path: &std::path::Path) -> bool {
    use std::fs;
    use std::io::Read;

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }

    // ELF magic: 0x7F 'E' 'L' 'F'
    magic == [0x7F, b'E', b'L', b'F']
}
