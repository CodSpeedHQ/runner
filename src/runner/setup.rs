use std::{
    env,
    process::{Command, Stdio},
};

use url::Url;

use super::{check_system::SystemInfo, helpers::download_file::download_file};
use crate::prelude::*;

const VALGRIND_CODSPEED_VERSION: &str = "3.21.0-0codspeed1";

/// Run a command with sudo if available
fn run_with_sudo(command_args: &[&str]) -> Result<()> {
    let use_sudo = Command::new("sudo")
        // `sudo true` will fail if sudo does not exist or the current user does not have sudo privileges
        .arg("true")
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let mut command_args: Vec<&str> = command_args.into();
    if use_sudo {
        command_args.insert(0, "sudo");
    }

    debug!("Running command: {}", command_args.join(" "));
    let output = Command::new(command_args[0])
        .args(&command_args[1..])
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| anyhow!("Failed to execute command: {}", command_args.join(" ")))?;

    if !output.status.success() {
        info!("stdout: {:?}", output.stdout);
        error!("stderr: {:?}", output.stderr);
        bail!("Failed to execute command: {}", command_args.join(" "));
    }

    Ok(())
}

pub async fn setup(system_info: &SystemInfo) -> Result<()> {
    let valgrind_deb_url = format!("https://github.com/CodSpeedHQ/valgrind-codspeed/releases/download/{}/valgrind_{}_ubuntu-{}_amd64.deb", VALGRIND_CODSPEED_VERSION, VALGRIND_CODSPEED_VERSION, system_info.os_version);
    let deb_path = env::temp_dir().join("valgrind-codspeed.deb");
    download_file(&Url::parse(valgrind_deb_url.as_str()).unwrap(), &deb_path).await?;

    run_with_sudo(&["apt-get", "update"])?;
    run_with_sudo(&["apt-get", "install", "-y", deb_path.to_str().unwrap()])?;

    info!("Environment ready");
    Ok(())
}
