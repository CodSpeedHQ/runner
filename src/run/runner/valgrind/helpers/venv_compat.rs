//! When creating a virtual environment, only `python3` is symlinked or copied which makes
//! lookups to `.venv/lib/libpython{version}.so.1.0` fail. This isn't an issue for most distributions
//! since they use absolute paths in the `python3` executable.
//!
//! However, uv uses relative paths which causes the lookups to fail:
//! ```no_run
//! > ldd .venv/bin/python3
//! /home/project/.venv/bin/../lib/libpython3.13.so.1.0 => not found
//! ```
//!
//! The solution to this is to add the symlink of the `libpython` shared object in the
//! virtual environment (`.venv/lib`) to make the symlink work correctly.

use crate::prelude::*;
use std::{io::Write, os::unix::fs::PermissionsExt, process::Command};

/// This scripts tries to find the virtual environment using `uv python find` and by finding the
/// `python3` executable in the activated virtual environment.
const VENV_COMPAT_SCRIPT: &str = include_str!("venv_compat.sh");

pub fn symlink_libpython(cwd: Option<&String>) -> anyhow::Result<()> {
    let rwx = std::fs::Permissions::from_mode(0o777);
    let mut script_file = tempfile::Builder::new()
        .suffix(".sh")
        .permissions(rwx)
        .tempfile()?;
    script_file.write_all(VENV_COMPAT_SCRIPT.as_bytes())?;

    let mut cmd = Command::new("bash");
    cmd.arg(script_file.path());

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    debug!("Running the venv compat script");
    let output = cmd.output()?;

    let stdout = String::from_utf8(output.stdout)?;
    debug!("Script output: {stdout}");

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        bail!("Failed to execute script: {stdout} {stderr}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Only run in Github Actions, to ensure python is dynamically linked.
    #[test_with::env(GITHUB_ACTIONS)]
    #[test]
    fn test_venv_compat_no_crash() {
        assert!(symlink_libpython(None).is_ok());
    }
}
