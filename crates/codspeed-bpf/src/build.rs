//! Build utilities for BPF programs using codspeed-bpf
//!
//! This module provides helpers for building BPF programs that use codspeed-bpf headers.
//!
//! To use this in your build.rs, add codspeed-bpf with the "build" feature to your
//! [build-dependencies]:
//!
//! ```toml
//! [build-dependencies]
//! codspeed-bpf = { path = "../codspeed-bpf", features = ["build"] }
//! ```
//!
//! Then in your build.rs:
//!
//! ```ignore
//! fn main() {
//!     codspeed_bpf::build::build_bpf("exectrack", "src/ebpf/c/exectrack.bpf.c");
//!     codspeed_bpf::build::generate_bindings("wrapper.h");
//! }
//! ```

use std::{env, path::PathBuf};

/// Build a BPF program with codspeed-bpf headers available
///
/// # Arguments
/// * `program_name` - The name of the BPF program (e.g., "exectrack", "memtrack")
/// * `source_file` - The path to the BPF source file (e.g., "src/ebpf/c/exectrack.bpf.c")
///
/// This function will:
/// 1. Compile the BPF program using libbpf-cargo
/// 2. Include codspeed-bpf headers in the clang search path
/// 3. Generate a skeleton into OUT_DIR/{program_name}.skel.rs
pub fn build_bpf(program_name: &str, source_file: &str) {
    use libbpf_cargo::SkeletonBuilder;

    println!("cargo:rerun-if-changed=src/ebpf/c");

    let arch = env::var("CARGO_CFG_TARGET_ARCH")
        .expect("CARGO_CFG_TARGET_ARCH must be set in build script");

    let output =
        PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!("{}.skel.rs", program_name));

    // Get the path to codspeed-bpf's C headers
    let codspeed_bpf_include = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .join("codspeed-bpf/src/c");

    SkeletonBuilder::new()
        .source(source_file)
        .clang_args([
            "-I",
            &vmlinux::include_path_root().join(arch).to_string_lossy(),
            "-I",
            &codspeed_bpf_include.to_string_lossy(),
        ])
        .build_and_generate(&output)
        .expect(&format!("Failed to build {}.bpf.c", program_name));
}

/// Generate Rust bindings for a C header file
///
/// # Arguments
/// * `header_file` - The path to the header file (e.g., "wrapper.h")
///
/// This function will:
/// 1. Use bindgen to generate Rust bindings
/// 2. Write the output to OUT_DIR/event.rs
pub fn generate_bindings(header_file: &str) {
    let bindings = bindgen::Builder::default()
        .header(header_file)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_file = PathBuf::from(env::var("OUT_DIR").unwrap()).join("event.rs");
    std::fs::write(&out_file, bindings.to_string()).expect("Couldn't write bindings!");
}
