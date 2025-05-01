use crate::{prelude::*, run::runner::helpers::setup::run_with_sudo};
use std::process::Command;

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
    debug!("Perf version: {:?}", version_str);

    version_str.is_ok()
}

pub fn install_perf() -> Result<()> {
    if is_perf_installed() {
        info!("Perf is already installed, skipping installation");
        return Ok(());
    }

    let cmd = Command::new("uname")
        .arg("-r")
        .output()
        .expect("Failed to execute uname");
    let kernel_release = String::from_utf8_lossy(&cmd.stdout);
    debug!("Kernel release: {}", kernel_release.trim());

    debug!("Installing perf");
    run_with_sudo(&["apt-get", "update"])?;
    run_with_sudo(&[
        "apt-get",
        "install",
        "--allow-downgrades",
        "-y",
        "linux-tools-common",
        "linux-tools-generic",
        &format!("linux-tools-{}", kernel_release.trim()),
    ])?;

    info!("Perf installation completed successfully");

    Ok(())
}
