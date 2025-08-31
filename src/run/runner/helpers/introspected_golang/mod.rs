use crate::prelude::*;
use std::{env, fs::File, io::Write, os::unix::fs::PermissionsExt, path::PathBuf};

const INTROSPECTED_GO_SCRIPT: &str = include_str!("go.sh");

/// Creates the `go` script that will replace the `go` binary while running
/// Returns the path to the script folder, which should be added to the PATH environment variable
pub fn setup() -> Result<PathBuf> {
    let script_folder = env::temp_dir().join("codspeed_introspected_go");
    std::fs::create_dir_all(&script_folder)?;
    let script_path = script_folder.join("go");
    let mut script_file = File::create(script_path)?;
    script_file.write_all(INTROSPECTED_GO_SCRIPT.as_bytes())?;
    // Make the script executable
    let mut perms = script_file.metadata()?.permissions();
    perms.set_mode(0o755);
    script_file.set_permissions(perms)?;
    Ok(script_folder)
}
