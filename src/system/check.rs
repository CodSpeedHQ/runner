use lazy_static::lazy_static;
use std::collections::HashSet;

use crate::prelude::*;

use super::SystemInfo;

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
    debug!("System info: {system_info:#?}");

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
