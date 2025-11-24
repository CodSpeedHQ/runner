use crate::prelude::*;

use std::{ffi::OsString, process::Command};

const FIND_PERF_CMD: &str =
    "find /usr/lib -executable -path \"/usr/lib/linux-tools-*/perf\" | sort | tail -n1";

/// Attempts to find the path to the `perf` executable that is installed and working.
/// Returns None if `perf` is not installed or not functioning correctly.
pub fn get_working_perf_executable() -> Option<OsString> {
    let is_installed = Command::new("which")
        .arg("perf")
        .output()
        .is_ok_and(|output| output.status.success());
    if !is_installed {
        debug!("perf is not installed");
        return None;
    }

    debug!("perf is installed, checking if it is functioning correctly");
    if Command::new("perf")
        .arg("--version") // here we use --version to check if perf is working
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return Some("perf".into());
    } else {
        // The following is a workaround for this outstanding Ubuntu issue: https://bugs.launchpad.net/ubuntu/+source/linux-hwe-6.14/+bug/2117159/
        debug!(
            "perf command is not functioning correctly, trying to find alternative path using \"{FIND_PERF_CMD}\""
        );
        if let Ok(perf_path) = Command::new("sh").arg("-c").arg(FIND_PERF_CMD).output() {
            if perf_path.status.success() {
                let path = String::from_utf8_lossy(&perf_path.stdout)
                    .trim()
                    .to_string();
                if path.is_empty() {
                    debug!("No alternative perf path found");
                    return None;
                }
                debug!("Found perf path: {path}");
                // Check if this perf is working by getting its version
                if let Ok(version_output) = Command::new(&path).arg("--version").output() {
                    if !version_output.status.success() {
                        debug!(
                            "Failed to get perf version from alternative path. stderr: {}",
                            String::from_utf8_lossy(&version_output.stderr)
                        );
                        return None;
                    }

                    let version = String::from_utf8_lossy(&version_output.stdout)
                        .trim()
                        .to_string();
                    debug!("Found perf version from alternative path: {version}");
                    return Some(path.into());
                }
            }
        }
    }

    debug!("perf is installed but not functioning correctly");
    None
}
