#[cfg(feature = "ebpf")]
fn main() {
    codspeed_bpf::build::build_bpf("memtrack", "src/ebpf/c/memtrack.bpf.c");
    codspeed_bpf::build::generate_bindings("wrapper.h");
}

#[cfg(not(feature = "ebpf"))]
fn main() {}
