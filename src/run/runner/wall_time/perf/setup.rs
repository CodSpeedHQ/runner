use crate::run::runner::helpers::apt;
use crate::{prelude::*, run::check_system::SystemInfo};

use std::{path::Path, process::Command};

fn cmd_version(cmd: &str) -> anyhow::Result<String> {
    let is_installed = Command::new("which")
        .arg(cmd)
        .output()
        .is_ok_and(|output| output.status.success());
    if !is_installed {
        bail!("{cmd} is not installed")
    }

    Ok(Command::new(cmd)
        .arg("--version")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())?)
}

fn is_perf_installed() -> bool {
    let version_str = cmd_version("perf");
    debug!("Perf version: {version_str:?}");

    version_str.is_ok()
}

pub async fn install_perf(system_info: &SystemInfo, setup_cache_dir: Option<&Path>) -> Result<()> {
    apt::install_cached(system_info, setup_cache_dir, is_perf_installed, || async {
        debug!("Installing perf");
        let cmd = Command::new("uname")
            .arg("-r")
            .output()
            .expect("Failed to execute uname");
        let kernel_release = String::from_utf8_lossy(&cmd.stdout);
        let kernel_release = kernel_release.trim();
        let linux_tools_kernel_release = format!("linux-tools-{kernel_release}");

        let packages = vec![
            "linux-tools-common".to_string(),
            "linux-tools-generic".to_string(),
            linux_tools_kernel_release,
        ];
        let package_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();

        apt::install(system_info, &package_refs)?;

        // Return package names for caching
        Ok(packages)
    })
    .await?;

    Ok(())
}
