use std::process::Command;

use serde::{Deserialize, Serialize};
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
    fn new() -> Result<Self> {
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

/// Checks if the system is supported and returns the system info
///
/// Supported systems:
/// - Ubuntu 20.04 on x86_64
/// - Ubuntu 22.04 on x86_64
/// - Debian 11 on x86_64
/// - Debian 12 on x86_64
pub fn check_system() -> Result<SystemInfo> {
    let system_info = SystemInfo::new()?;
    debug!("System info: {:#?}", system_info);

    match (system_info.os.as_str(), system_info.os_version.as_str()) {
        ("Ubuntu", "20.04") | ("Ubuntu", "22.04") | ("Debian", "11") | ("Debian", "12") => (),
        ("Ubuntu", _) => bail!("Only Ubuntu 20.04 and 22.04 are supported at the moment"),
        ("Debian", _) => bail!("Only Debian 11 and 12 are supported at the moment"),
        _ => bail!("Only Ubuntu and Debian are supported at the moment"),
    }
    if system_info.arch != "x86_64" {
        bail!("Only x86_64 is supported at the moment");
    }

    Ok(system_info)
}
