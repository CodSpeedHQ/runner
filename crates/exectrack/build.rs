fn main() {
    codspeed_bpf::build::build_bpf("exectrack", "src/ebpf/c/exectrack.bpf.c");
    codspeed_bpf::build::generate_bindings("wrapper.h");
}
