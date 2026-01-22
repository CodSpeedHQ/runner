use crate::prelude::*;

use std::io::Write;
use std::sync::OnceLock;

/// Filename for the preload shared library.
const PRELOAD_LIB_FILENAME: &str = env!("CODSPEED_PRELOAD_LIB_FILENAME");

/// The preload library binary embedded at compile time.
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
        .suffix(PRELOAD_LIB_FILENAME)
        .tempfile()
        .context("Failed to create temp file for preload library")?;

    file.write_all(PRELOAD_LIB_BYTES)
        .context("Failed to write preload library to temp file")?;

    debug!(
        "Extracted preload library to temp file: {}",
        file.path().display()
    );

    Ok(file)
}

/// Returns the path to the preload library, extracting it to a temp file if needed.
pub(super) fn get_preload_lib_path() -> Result<&'static std::path::Path> {
    if let Some(file) = PRELOAD_LIB_FILE.get() {
        return Ok(file.path());
    }

    let file = extract_preload_lib()?;
    Ok(PRELOAD_LIB_FILE.get_or_init(|| file).path())
}
