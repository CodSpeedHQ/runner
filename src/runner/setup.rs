use std::{env, process::Command};

use url::Url;

use super::{check_system::SystemInfo, helpers::download_file::download_file};
use crate::prelude::*;

const VALGRIND_CODSPEED_VERSION: &str = "3.21.0-0codspeed1";

pub async fn setup(system_info: &SystemInfo) -> Result<()> {
    let valgrind_deb_url = format!("https://github.com/CodSpeedHQ/valgrind-codspeed/releases/download/{}/valgrind_{}_ubuntu-{}_amd64.deb", VALGRIND_CODSPEED_VERSION, VALGRIND_CODSPEED_VERSION, system_info.os_version);
    let deb_path = env::temp_dir().join("valgrind-codspeed.deb");
    download_file(&Url::parse(valgrind_deb_url.as_str()).unwrap(), &deb_path).await?;
    let update_status = Command::new("sudo")
        .args(["apt", "update"])
        .status()
        .map_err(|_| anyhow!("Failed to update apt"))?;
    if !update_status.success() {
        bail!("Failed to update apt");
    }
    let install_status = Command::new("sudo")
        .args(["apt", "install", "-y", deb_path.to_str().unwrap()])
        .status()
        .map_err(|_| anyhow!("Failed to install valgrind-codspeed"))?;
    if !install_status.success() {
        bail!("Failed to install valgrind-codspeed");
    }
    Ok(())
}
