use std::{
    env,
    process::{Command, Stdio},
};

use url::Url;

use super::{check_system::SystemInfo, helpers::download_file::download_file};
use crate::{config::Config, prelude::*};

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

async fn install_valgrind(system_info: &SystemInfo) -> Result<()> {
    debug!("Installing valgrind");
    let valgrind_deb_url = format!("https://github.com/CodSpeedHQ/valgrind-codspeed/releases/download/{}/valgrind_{}_ubuntu-{}_amd64.deb", VALGRIND_CODSPEED_VERSION, VALGRIND_CODSPEED_VERSION, system_info.os_version);
    let deb_path = env::temp_dir().join("valgrind-codspeed.deb");
    download_file(&Url::parse(valgrind_deb_url.as_str()).unwrap(), &deb_path).await?;

    run_with_sudo(&["apt-get", "update"])?;
    run_with_sudo(&["apt-get", "install", "-y", deb_path.to_str().unwrap()])?;

    Ok(())
}

const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";

async fn install_mongodb_tracer() -> Result<()> {
    debug!("Installing mongodb-tracer");
    // TODO: release the tracer and update this url
    let installer_url = format!("https://codspeed-public-assets.s3.eu-west-1.amazonaws.com/mongo-tracer/{MONGODB_TRACER_VERSION}/cs-mongo-tracer-installer.sh");
    let installer_path = env::temp_dir().join("cs-mongo-tracer-installer.sh");
    download_file(
        &Url::parse(installer_url.as_str()).unwrap(),
        &installer_path,
    )
    .await?;

    let output = Command::new("bash")
        .arg(installer_path.to_str().unwrap())
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| anyhow!("Failed to install mongo-tracer"))?;

    if !output.status.success() {
        info!("stdout: {:?}", output.stdout);
        error!("stderr: {:?}", output.stderr);
        bail!("Failed to install mongo-tracer");
    }

    Ok(())
}

pub async fn setup(system_info: &SystemInfo, config: &Config) -> Result<()> {
    install_valgrind(system_info).await?;

    if config.instruments.is_mongodb_enabled() {
        install_mongodb_tracer().await?;
    }

    info!("Environment ready");
    Ok(())
}
