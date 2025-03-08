use std::process::Command;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

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
    pub cpu_brand: String,
    pub cpu_name: String,
    pub cpu_vendor_id: String,
    pub cpu_cores: usize,
    pub total_memory_gb: u64,
}

#[cfg(test)]
impl SystemInfo {
    pub fn test() -> Self {
        SystemInfo {
            os: "ubuntu".to_string(),
            os_version: "20.04".to_string(),
            arch: "x86_64".to_string(),
            host: "host".to_string(),
            user: "user".to_string(),
            cpu_brand: "Intel(R) Xeon(R) CPU E5-2686 v4 @ 2.30GHz".to_string(),
            cpu_name: "cpu0".to_string(),
            cpu_vendor_id: "GenuineIntel".to_string(),
            cpu_cores: 2,
            total_memory_gb: 8,
        }
    }
}

impl SystemInfo {
    pub fn new() -> Result<Self> {
        let os = System::distribution_id();
        let os_version = System::os_version().ok_or(anyhow!("Failed to get OS version"))?;
        let arch = System::cpu_arch();
        let user = get_user()?;
        let host = System::host_name().ok_or(anyhow!("Failed to get host name"))?;

        let s = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let cpu_cores = s
            .physical_core_count()
            .ok_or(anyhow!("Failed to get CPU core count"))?;
        let total_memory_gb = s.total_memory().div_ceil(1024_u64.pow(3));

        // take the first CPU to get the brand, name and vendor id
        let cpu = s
            .cpus()
            .iter()
            .next()
            .ok_or(anyhow!("Failed to get CPU info"))?;
        let cpu_brand = cpu.brand().to_string();
        let cpu_name = cpu.name().to_string();
        let cpu_vendor_id = cpu.vendor_id().to_string();

        Ok(SystemInfo {
            os,
            os_version,
            arch,
            host,
            user,
            cpu_brand,
            cpu_name,
            cpu_vendor_id,
            cpu_cores,
            total_memory_gb,
        })
    }
}

lazy_static! {
    static ref SUPPORTED_SYSTEMS: HashSet<(&'static str, &'static str, &'static str)> = {
        HashSet::from([
            ("ubuntu", "22.04", "x86_64"),
            ("ubuntu", "24.04", "x86_64"),
            ("ubuntu", "22.04", "aarch64"),
            ("ubuntu", "24.04", "aarch64"),
            ("debian", "12", "x86_64"),
            ("debian", "12", "aarch64"),
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

    match system_info.arch.as_str() {
        "x86_64" | "aarch64" => {
            warn!(
                "Unofficially supported system: {} {}. Continuing with best effort support.",
                system_info.os, system_info.os_version
            );
            return Ok(());
        }
        _ => {}
    }

    bail!(
        "Unsupported system: {} {}",
        system_info.os,
        system_info.os_version
    );
}
