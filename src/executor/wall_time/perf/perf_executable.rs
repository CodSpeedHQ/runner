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

/// Detects if the required perf events are available on this system.
/// Returns the flags to pass to perf record command if they are available, otherwise returns None.
pub fn get_event_flags(perf_executable: &OsString) -> anyhow::Result<Option<String>> {
    const CYCLES_EVENT_NAME: &str = "cycles";
    const CACHE_REFERENCES_EVENT_NAME: &str = "cache-references";
    const CACHE_MISSES_EVENT_NAME: &str = "cache-misses";

    let perf_events = [
        CYCLES_EVENT_NAME,
        CACHE_REFERENCES_EVENT_NAME,
        CACHE_MISSES_EVENT_NAME,
    ];

    let output = Command::new(perf_executable)
        .arg("list")
        .output()
        .context("Failed to run perf list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if all required events are available
    // Expected format in `perf list` output:
    //
    // List of pre-defined events (to be used in -e or -M):
    //
    //  branch-instructions OR branches                    [Hardware event]
    //  branch-misses                                      [Hardware event]
    //  bus-cycles                                         [Hardware event]
    //  cache-misses                                       [Hardware event]
    //  cache-references                                   [Hardware event]
    //  cpu-cycles OR cycles                               [Hardware event]
    //  instructions                                       [Hardware event]
    //  ref-cycles                                         [Hardware event]
    let missing_events: Vec<&str> = perf_events
        .iter()
        .filter(|&&event| {
            !stdout
                .lines()
                .any(|line| line.split_whitespace().any(|word| word == event))
        })
        .copied()
        .collect();

    if !missing_events.is_empty() {
        debug!(
            "Not all required perf events available. Missing: [{}], using default events",
            missing_events.join(", ")
        );
        return Ok(None);
    }

    debug!(
        "All required perf events available: {}",
        perf_events.join(", ")
    );
    Ok(Some(format!("-e {{{}}}", perf_events.join(","))))
}
