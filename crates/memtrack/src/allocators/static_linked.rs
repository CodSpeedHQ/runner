use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::allocators::{AllocatorKind, AllocatorLib};

/// Walk upward from current directory to find build directories.
/// Returns all found build directories in order of preference.
fn find_build_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Ok(mut current_dir) = std::env::current_dir() else {
        return dirs;
    };

    loop {
        // Check for Cargo/Rust build directory
        let cargo_analysis = current_dir.join("target").join("codspeed").join("analysis");
        if cargo_analysis.is_dir() {
            dirs.push(cargo_analysis);
        }

        // Check for Bazel build directory
        let bazel_bin = current_dir.join("bazel-bin");
        if bazel_bin.is_dir() {
            dirs.push(bazel_bin);
        }

        // Check for CMake build directory
        let cmake_build = current_dir.join("build");
        if cmake_build.is_dir() {
            dirs.push(cmake_build);
        }

        if !current_dir.pop() {
            break;
        }
    }

    dirs
}

fn find_binaries_in_dir(dir: &Path) -> Vec<PathBuf> {
    glob::glob(&format!("{}/**/*", dir.display()))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|p| p.is_file() && super::is_elf(p))
        .collect::<Vec<_>>()
}

fn find_statically_linked_allocator(path: &Path) -> Option<AllocatorKind> {
    use object::{Object, ObjectSymbol};

    let data = fs::read(path).ok()?;
    let file = object::File::parse(&*data).ok()?;

    let symbols: HashSet<_> = file
        .symbols()
        .chain(file.dynamic_symbols())
        .filter(|s| s.is_definition())
        .filter_map(|s| s.name().ok())
        .collect();

    // FIXME: We don't support multiple statically linked allocators for now

    AllocatorKind::all()
        .iter()
        .find(|kind| kind.symbols().iter().any(|s| symbols.contains(s)))
        .copied()
}

pub fn find_all() -> anyhow::Result<Vec<AllocatorLib>> {
    let build_dirs = find_build_dirs();
    if build_dirs.is_empty() {
        return Ok(vec![]);
    }

    let mut allocators = Vec::new();
    for build_dir in build_dirs {
        let bins = find_binaries_in_dir(&build_dir);

        for bin in bins {
            let Some(kind) = find_statically_linked_allocator(&bin) else {
                continue;
            };

            allocators.push(AllocatorLib { kind, path: bin });
        }
    }

    Ok(allocators)
}
