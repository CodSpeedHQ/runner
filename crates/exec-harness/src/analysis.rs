use crate::prelude::*;

use crate::BenchmarkCommand;
use crate::constants;
use crate::uri;
use codspeed::instrument_hooks::InstrumentHooks;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::OnceLock;

pub fn perform(commands: Vec<BenchmarkCommand>) -> Result<()> {
    let hooks = InstrumentHooks::instance();

    for benchmark_cmd in commands {
        let name_and_uri = uri::generate_name_and_uri(&benchmark_cmd.name, &benchmark_cmd.command);
        name_and_uri.print_executing();

        let mut cmd = Command::new(&benchmark_cmd.command[0]);
        cmd.args(&benchmark_cmd.command[1..]);
        hooks.start_benchmark().unwrap();
        let status = cmd.status();
        hooks.stop_benchmark().unwrap();
        let status = status.context("Failed to execute command")?;

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }

        hooks.set_executed_benchmark(&name_and_uri.uri).unwrap();
    }

    Ok(())
}

/// Filename for the preload shared library.
const PRELOAD_LIB_FILENAME: &str = env!("CODSPEED_PRELOAD_LIB_FILENAME");

/// The preload library binary embedded at compile time.
/// This library is used for LD_PRELOAD-based instrumentation injection.
const PRELOAD_LIB_BYTES: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/",
    env!("CODSPEED_PRELOAD_LIB_FILENAME")
));

/// Lazily initialized temp file containing the extracted preload library.
/// Kept in a static to prevent cleanup until process exit.
static PRELOAD_LIB_FILE: OnceLock<tempfile::NamedTempFile> = OnceLock::new();

/// Extracts the preload library to a temp file.
fn extract_preload_lib() -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix(PRELOAD_LIB_FILENAME)
        .tempfile()
        .context("Failed to create temp file for preload library")?;

    file.write_all(PRELOAD_LIB_BYTES)
        .context("Failed to write preload library to temp file")?;

    // Make the library executable
    let mut permissions = file
        .as_file()
        .metadata()
        .context("Failed to get temp file metadata")?
        .permissions();
    permissions.set_mode(0o755);
    file.as_file()
        .set_permissions(permissions)
        .context("Failed to set temp file permissions")?;

    Ok(file)
}

/// Returns the path to the preload library, extracting it to a temp file if needed.
fn get_preload_lib_path() -> Result<&'static std::path::Path> {
    if let Some(file) = PRELOAD_LIB_FILE.get() {
        return Ok(file.path());
    }

    let file = extract_preload_lib()?;
    Ok(PRELOAD_LIB_FILE.get_or_init(|| file).path())
}

pub fn perform_with_valgrind(commands: Vec<BenchmarkCommand>) -> Result<()> {
    let preload_lib_path = get_preload_lib_path()?;

    for benchmark_cmd in commands {
        let name_and_uri = uri::generate_name_and_uri(&benchmark_cmd.name, &benchmark_cmd.command);
        name_and_uri.print_executing();

        let mut cmd = Command::new(&benchmark_cmd.command[0]);
        cmd.args(&benchmark_cmd.command[1..]);
        // Use LD_PRELOAD to inject instrumentation into the child process
        cmd.env("LD_PRELOAD", preload_lib_path);
        cmd.env(constants::URI_ENV, &name_and_uri.uri);

        let status = cmd.status().context("Failed to execute command")?;

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }
    }

    Ok(())
}
