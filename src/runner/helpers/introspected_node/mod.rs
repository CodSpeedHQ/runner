use crate::prelude::*;
use std::{env, fs::File, io::Write, os::unix::fs::PermissionsExt, path::PathBuf};

const INTROSPECTED_NODE_SCRIPT: &str = include_str!("node.sh");

/// Creates the node script that will replace the node binary while running
/// Returns the path to the script
pub fn setup_introspected_node() -> Result<PathBuf> {
    let script_folder = env::temp_dir().join("codspeed_introspected_node");
    std::fs::create_dir_all(&script_folder)?;
    let script_path = script_folder.join("node");
    let mut script_file = File::create(&script_path)?;
    script_file.write_all(INTROSPECTED_NODE_SCRIPT.as_bytes())?;
    // Make the script executable
    let mut perms = script_file.metadata()?.permissions();
    perms.set_mode(0o755);
    script_file.set_permissions(perms)?;
    Ok(script_path)
}
