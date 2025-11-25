fn main() {
    prost_build::compile_protos(&["proto/benchmark_results.proto"], &["proto"])
        .expect("Failed to compile protobuf");
}
