//! Static allocator detection for binaries in `target/analysis`.
//!
//! This module provides functionality to detect statically linked allocators
//! by scanning ELF binaries for exported allocator symbols.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::allocators::{AllocatorKind, AllocatorLib};

const LIBC_SYMBOLS: &[&str] = &["malloc", "free", "realloc", "calloc"];
const JEMALLOC_SYMBOLS: &[&str] = &["je_malloc", "je_free", "je_realloc", "je_calloc"];
const MIMALLOC_SYMBOLS: &[&str] = &["mi_malloc", "mi_free", "mi_realloc", "mi_calloc"];

/// Walk upward from current directory to find `target/analysis`.
pub fn find_analysis_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        let candidate = dir.join("target").join("analysis");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Check if a file is an ELF binary by reading the magic bytes.
fn is_elf(path: &Path) -> bool {
    let mut f = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::debug!("Failed to open {:?}: {e}", path);
            return false;
        }
    };
    let mut magic = [0u8; 4];
    if let Err(e) = f.read_exact(&mut magic) {
        log::debug!("Failed to read magic from {:?}: {e}", path);
        return false;
    }
    magic == [0x7F, b'E', b'L', b'F']
}

/// Recursively walk a directory and find all ELF binaries.
fn walk_elf_binaries(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                log::debug!("Skipping dir {:?}: {e}", dir);
                continue;
            }
        };

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    log::debug!("Skipping bad dir entry in {:?}: {e}", dir);
                    continue;
                }
            };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else {
                continue;
            };

            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() && is_elf(&path) {
                result.push(path);
            }
        }
    }

    result
}

/// Detect which allocator kind is statically linked in an ELF binary.
fn detect_allocators_in_elf(path: &Path) -> Option<AllocatorKind> {
    use object::{Object, ObjectSymbol};

    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            log::debug!("Failed to read {:?}: {e}", path);
            return None;
        }
    };

    let file = match object::File::parse(&*data) {
        Ok(f) => f,
        Err(e) => {
            log::debug!("Failed to parse ELF {:?}: {e}", path);
            return None;
        }
    };

    let mut has_libc = false;
    let mut has_jemalloc = false;
    let mut has_mimalloc = false;

    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        if !symbol.is_definition() {
            continue;
        }

        let Ok(name) = symbol.name() else {
            continue;
        };

        if LIBC_SYMBOLS.contains(&name) {
            has_libc = true;
        }
        if JEMALLOC_SYMBOLS.contains(&name) {
            has_jemalloc = true;
        }
        if MIMALLOC_SYMBOLS.contains(&name) {
            has_mimalloc = true;
        }
    }

    if has_jemalloc {
        Some(AllocatorKind::Jemalloc)
    } else if has_mimalloc {
        Some(AllocatorKind::Mimalloc)
    } else if has_libc {
        Some(AllocatorKind::Libc)
    } else {
        None
    }
}

/// Scan `target/analysis` for binaries with statically linked allocators.
pub fn scan_analysis_for_allocators() -> Vec<AllocatorLib> {
    let Some(analysis_dir) = find_analysis_dir() else {
        log::debug!("No target/analysis directory found");
        return Vec::new();
    };

    log::debug!("Scanning {:?} for static allocators", analysis_dir);

    let binaries = walk_elf_binaries(&analysis_dir);
    let mut result = Vec::new();

    for bin in binaries {
        if let Some(kind) = detect_allocators_in_elf(&bin) {
            log::debug!("Found {:?} allocator in {:?}", kind, bin);
            result.push(AllocatorLib { kind, path: bin });
        }
    }

    result.sort_by_key(|lib| (lib.kind as u8, lib.path.clone()));
    result.dedup_by(|a, b| a.kind == b.kind && a.path == b.path);

    result
}
