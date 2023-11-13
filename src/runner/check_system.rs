use std::process::Command;

use crate::prelude::*;

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

pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
}

pub fn check_system() -> Result<SystemInfo> {
    let (os, os_version) = get_os_details()?;
    debug!("OS: {}, Version: {}", os, os_version);
    if os != "Ubuntu" {
        bail!("Only Ubuntu is supported at the moment");
    }
    if !["20.04", "22.04"].contains(&os_version.as_str()) {
        bail!("Only Ubuntu 20.04 and 22.04 are supported at the moment");
    }
    let arch = get_arch()?;
    debug!("Arch: {}", arch);
    if arch != "amd64" {
        bail!("Only amd64 is supported at the moment");
    }
    Ok(SystemInfo {
        os,
        os_version,
        arch,
    })
}
