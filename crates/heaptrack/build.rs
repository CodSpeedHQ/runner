use std::{env, path::PathBuf};

use libbpf_cargo::SkeletonBuilder;

fn main() {
    println!("cargo:rerun-if-changed=src/bpf");

    // Build the BPF program
    let arch = env::var("CARGO_CFG_TARGET_ARCH")
        .expect("CARGO_CFG_TARGET_ARCH must be set in build script");
    let out_dir =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set in build script"));
    let heaptrack_out = out_dir.join("heaptrack.skel.rs");
    SkeletonBuilder::new()
        .source("src/bpf/heaptrack.bpf.c")
        .clang_args([
            "-I",
            &vmlinux::include_path_root().join(arch).to_string_lossy(),
        ])
        .build_and_generate(&heaptrack_out)
        .unwrap();

    // Generate bindings for event.h
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_file = out_dir.join("event.rs");
    std::fs::write(&out_file, bindings.to_string()).expect("Couldn't write bindings!");
}
