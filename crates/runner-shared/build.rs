fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("Failed to get protoc path");
    prost_build::Config::new()
        .protoc_executable(protoc)
        .compile_protos(&["proto/benchmark_results.proto"], &["proto"])
        .expect("Failed to compile protobuf");
}
