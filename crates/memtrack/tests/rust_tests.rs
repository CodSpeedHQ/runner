#[macro_use]
mod shared;

use memtrack::AllocatorLib;
use rstest::rstest;
use std::path::Path;
use std::process::Command;

fn compile_rust_crate(
    crate_dir: &Path,
    name: &str,
    features: &[&str],
) -> anyhow::Result<std::path::PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(crate_dir)
        .args(["build", "--release", "--bin", name]);

    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    let output = cmd.output()?;
    if !output.status.success() {
        eprintln!("cargo stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("cargo stdout: {}", String::from_utf8_lossy(&output.stdout));
        return Err(anyhow::anyhow!("Failed to compile Rust crate"));
    }

    let binary_path = crate_dir.join(format!("target/release/{name}"));
    Ok(binary_path)
}

#[rstest]
#[case("system", &[])]
#[case("jemalloc", &["with-jemalloc"])]
#[case("mimalloc", &["with-mimalloc"])]
#[test_log::test]
fn test_rust_alloc_tracking(
    #[case] name: &str,
    #[case] features: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let crate_path = Path::new("testdata/alloc_rust");
    let binary = compile_rust_crate(crate_path, "alloc_rust", features)?;

    // Try to find a static allocator in the binary, then attach to it as well
    // This is needed because the CWD is different, which breaks the heuristics.
    let allocators = AllocatorLib::from_path_static(&binary)
        .map(|a| vec![a])
        .unwrap_or_default();

    let (events, thread_handle) = shared::track_binary_with_opts(&binary, &allocators)?;
    assert_events_with_marker!(name, &events);

    thread_handle.join().unwrap();
    Ok(())
}
