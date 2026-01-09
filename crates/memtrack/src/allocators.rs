//! Generic allocator discovery infrastructure.
//!
//! This module provides a framework for discovering and attaching to different
//! memory allocators. It's designed to be easily extensible for adding new allocators.

use std::path::PathBuf;

/// Represents the different allocator types we support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocatorKind {
    /// Standard C library (glibc, musl, etc.)
    Libc,
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
            AllocatorKind::Jemalloc,
            AllocatorKind::Mimalloc,
        ]
    }

    /// Returns the glob patterns used to find this allocator's shared libraries.
    pub fn glob_patterns(&self) -> &'static [&'static str] {
        match self {
            AllocatorKind::Libc => &[
                // Debian, Ubuntu: Standard Linux multiarch paths
                "/lib/*-linux-gnu/libc.so.6",
                "/usr/lib/*-linux-gnu/libc.so.6",
                // RHEL, Fedora, CentOS, Arch
                "/lib*/libc.so.6",
                "/usr/lib*/libc.so.6",
                // NixOS: find all glibc versions in the Nix store
                "/nix/store/*glibc*/lib/libc.so.6",
            ],
            AllocatorKind::Jemalloc => &[
                // Debian, Ubuntu: Standard Linux multiarch paths
                "/lib/*-linux-gnu/libjemalloc.so*",
                "/usr/lib/*-linux-gnu/libjemalloc.so*",
                // RHEL, Fedora, CentOS, Arch
                "/lib*/libjemalloc.so*",
                "/usr/lib*/libjemalloc.so*",
                "/usr/local/lib*/libjemalloc.so*",
                // NixOS
                "/nix/store/*jemalloc*/lib/libjemalloc.so*",
            ],
            AllocatorKind::Mimalloc => &[
                // Debian, Ubuntu: Standard Linux multiarch paths
                "/lib/*-linux-gnu/libmimalloc.so*",
                "/usr/lib/*-linux-gnu/libmimalloc.so*",
                // RHEL, Fedora, CentOS, Arch
                "/lib*/libmimalloc.so*",
                "/usr/lib*/libmimalloc.so*",
                "/usr/local/lib*/libmimalloc.so*",
                // NixOS
                "/nix/store/*mimalloc*/lib/libmimalloc.so*",
            ],
        }
    }

    /// Returns a human-readable name for the allocator.
    pub fn name(&self) -> &'static str {
        match self {
            AllocatorKind::Libc => "libc",
            AllocatorKind::Jemalloc => "jemalloc",
            AllocatorKind::Mimalloc => "mimalloc",
        }
    }

    /// Returns true if this allocator is required (must be found).
    pub fn is_required(&self) -> bool {
        matches!(self, AllocatorKind::Libc)
    }
}

/// Discovered allocator library with its kind and path.
#[derive(Debug, Clone)]
pub struct AllocatorLib {
    pub kind: AllocatorKind,
    pub path: PathBuf,
}

impl AllocatorLib {
    /// Find all allocator libraries (both dynamic and statically linked).
    pub fn find_all() -> anyhow::Result<Vec<Self>> {
        use std::collections::HashSet;

        let mut results = Self::find_all_dynamic()?;
        let static_bins = Self::find_all_static();

        let mut seen_paths: HashSet<PathBuf> = results.iter().map(|lib| lib.path.clone()).collect();
        for lib in static_bins {
            if seen_paths.insert(lib.path.clone()) {
                results.push(lib);
            }
        }

        results.sort_by_key(|lib| (lib.kind as u8, lib.path.clone()));
        Ok(results)
    }

    /// Find dynamically linked allocator libraries on the system.
    fn find_all_dynamic() -> anyhow::Result<Vec<Self>> {
        use std::collections::HashSet;

        let mut results = Vec::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        for kind in AllocatorKind::all() {
            let mut found_any = false;

            for pattern in kind.glob_patterns() {
                let paths = glob::glob(pattern)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p.ok())
                    .filter_map(|p| p.canonicalize().ok())
                    .filter(|path| {
                        std::fs::metadata(path)
                            .map(|m| m.is_file())
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();

                for path in paths {
                    if seen_paths.insert(path.clone()) {
                        results.push(AllocatorLib { kind: *kind, path });
                        found_any = true;
                    }
                }
            }

            if kind.is_required() && !found_any {
                anyhow::bail!("Could not find required allocator: {}", kind.name());
            }
        }

        results.sort_by_key(|lib| (lib.kind as u8, lib.path.clone()));
        Ok(results)
    }

    /// Find statically linked allocators in binaries under `target/analysis`.
    fn find_all_static() -> Vec<Self> {
        crate::allocators_static::scan_analysis_for_allocators()
    }
}

/// Detect which allocators a specific process is using by examining /proc/<pid>/maps.
#[allow(dead_code)]
pub fn detect_allocators_for_pid(
    pid: i32,
) -> anyhow::Result<std::collections::HashSet<AllocatorKind>> {
    use anyhow::Context;
    use std::collections::HashSet;

    let maps_path = format!("/proc/{pid}/maps");
    let contents = std::fs::read_to_string(&maps_path)
        .with_context(|| format!("Failed to read {maps_path}"))?;

    let mut allocs = HashSet::new();

    for line in contents.lines() {
        if let Some(path) = line.split_whitespace().last() {
            if !path.starts_with('/') {
                continue;
            }

            if path.contains("libc.so") || path.contains("libc-") {
                allocs.insert(AllocatorKind::Libc);
            }
            if path.contains("libjemalloc") {
                allocs.insert(AllocatorKind::Jemalloc);
            }
            if path.contains("libmimalloc") {
                allocs.insert(AllocatorKind::Mimalloc);
            }
        }
    }

    Ok(allocs)
}
