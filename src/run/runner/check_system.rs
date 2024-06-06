use std::process::Command;

use crate::prelude::*;

/// Returns the OS and version of the system
///
/// ## Example output
/// ```
/// ("Ubuntu", "20.04")
/// ("Ubuntu", "22.04")
/// ("Debian", "11")
/// ("Debian", "12")
/// ```
fn get_os_details() -> Result<(String, String)> {
    let lsb_output = Command::new("lsb_release")
        .args(["-i", "-r", "-s"])
        .output()
        .map_err(|_| anyhow!("Failed to get system info"))?;
    if !lsb_output.status.success() {
        bail!("Failed to get system info");
    }
    let output_str =
        String::from_utf8(lsb_output.stdout).map_err(|_| anyhow!("Failed to parse system info"))?;
    let mut lines = output_str.trim().lines();
    let os = lines
        .next()
        .ok_or_else(|| anyhow!("Failed to get OS info"))?;
    let os_version = lines
        .next()
        .ok_or_else(|| anyhow!("Failed to get OS version"))?;
    Ok((os.to_string(), os_version.to_string()))
}

/// NOTE: Since this relies on `dpkg` this will only work on Debian based systems
fn get_arch() -> Result<String> {
    let arch_output = Command::new("dpkg")
        .args(["--print-architecture"])
        .output()
        .map_err(|_| anyhow!("Failed to get architecture info"))?;
    if !arch_output.status.success() {
        bail!("Failed to get architecture info");
    }
    let output_str = String::from_utf8(arch_output.stdout)
        .map_err(|_| anyhow!("Failed to parse architecture info"))?;
    Ok(output_str.trim().to_string())
}

#[derive(Eq, PartialEq, Hash)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
}

/// Checks if the system is supported
///
/// Supported systems:
/// - Ubuntu 20.04 on amd64
/// - Ubuntu 22.04 on amd64
/// - Debian 11 on amd64
/// - Debian 12 on amd64
pub fn check_system() -> Result<SystemInfo> {
    let (os, os_version) = get_os_details()?;
    debug!("OS: {}, Version: {}", os, os_version);
    match (os.as_str(), os_version.as_str()) {
        ("Ubuntu", "20.04") | ("Ubuntu", "22.04") | ("Debian", "11") | ("Debian", "12") => (),
        ("Ubuntu", _) => bail!("Only Ubuntu 20.04 and 22.04 are supported at the moment"),
        ("Debian", _) => bail!("Only Debian 11 and 12 are supported at the moment"),
        _ => bail!("Only Ubuntu and Debian are supported at the moment"),
    }
    let arch = get_arch()?;
    debug!("Arch: {}", arch);
    if arch != "amd64" && arch != "arm64" {
        bail!("Only amd64 and arm64 are supported at the moment");
    }
    Ok(SystemInfo {
        os,
        os_version,
        arch,
    })
}
