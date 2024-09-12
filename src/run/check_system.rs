use std::process::Command;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use sysinfo::System;

use crate::prelude::*;

fn get_user() -> Result<String> {
    let user_output = Command::new("whoami")
        .output()
        .map_err(|_| anyhow!("Failed to get user info"))?;
    if !user_output.status.success() {
        bail!("Failed to get user info");
    }
    let output_str =
        String::from_utf8(user_output.stdout).map_err(|_| anyhow!("Failed to parse user info"))?;
    Ok(output_str.trim().to_string())
}

#[derive(Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub host: String,
    pub user: String,
}

#[cfg(test)]
impl SystemInfo {
    pub fn test() -> Self {
        SystemInfo {
            os: "Ubuntu".to_string(),
            os_version: "20.04".to_string(),
            arch: "x86_64".to_string(),
            host: "host".to_string(),
            user: "user".to_string(),
        }
    }
}

impl SystemInfo {
    pub fn new() -> Result<Self> {
        let os = System::name().ok_or(anyhow!("Failed to get OS name"))?;
        let os_version = System::os_version().ok_or(anyhow!("Failed to get OS version"))?;
        let arch = System::cpu_arch().ok_or(anyhow!("Failed to get CPU architecture"))?;
        let user = get_user()?;
        let host = System::host_name().ok_or(anyhow!("Failed to get host name"))?;

        Ok(SystemInfo {
            os,
            os_version,
            arch,
            host,
            user,
        })
    }
}

lazy_static! {
    static ref SUPPORTED_SYSTEMS: HashSet<(&'static str, &'static str, &'static str)> = {
        HashSet::from([
            ("Ubuntu", "20.04", "x86_64"),
            ("Ubuntu", "22.04", "x86_64"),
            ("Ubuntu", "22.04", "aarch64"),
            ("Debian", "11", "x86_64"),
            ("Debian", "12", "x86_64"),
        ])
    };
}

/// Checks if the provided system info is supported
///
/// Supported systems:
/// - Ubuntu 20.04 x86_64
/// - Ubuntu 22.04 x86_64 and aarch64
/// - Debian 11 x86_64
/// - Debian 12 x86_64
pub fn check_system(system_info: &SystemInfo) -> Result<()> {
    debug!("System info: {:#?}", system_info);

    let system_tuple = (
        system_info.os.as_str(),
        system_info.os_version.as_str(),
        system_info.arch.as_str(),
    );

    if SUPPORTED_SYSTEMS.contains(&system_tuple) {
        return Ok(());
    }

    bail!("Unsupported system: {:?}", system_info);
}
