use std::{
    env,
    process::{Command, Stdio},
};

use url::Url;

use super::helpers::download_file::download_file;
use crate::{prelude::*, MONGODB_TRACER_VERSION, VALGRIND_CODSPEED_VERSION};
use crate::{
    run::{check_system::SystemInfo, config::Config},
    VALGRIND_CODSPEED_DEB_VERSION,
};

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
        info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Failed to execute command: {}", command_args.join(" "));
    }

    Ok(())
}

fn get_codspeed_valgrind_filename(system_info: &SystemInfo) -> Result<String> {
    let (version, architecture) = match (
        system_info.os.as_str(),
        system_info.os_version.as_str(),
        system_info.arch.as_str(),
    ) {
        ("ubuntu", "22.04", "x86_64") | ("debian", "12", "x86_64") => ("22.04", "amd64"),
        ("ubuntu", "24.04", "x86_64") => ("24.04", "amd64"),
        ("ubuntu", "22.04", "aarch64") | ("debian", "12", "aarch64") => ("22.04", "arm64"),
        ("ubuntu", "24.04", "aarch64") => ("24.04", "arm64"),
        _ => bail!("Unsupported system"),
    };

    Ok(format!(
        "valgrind_{}_ubuntu-{}_{}.deb",
        VALGRIND_CODSPEED_DEB_VERSION.as_str(),
        version,
        architecture
    ))
}

fn is_valgrind_installed() -> bool {
    let is_valgrind_installed = Command::new("which")
        .arg("valgrind")
        .output()
        .is_ok_and(|output| output.status.success());
    if !is_valgrind_installed {
        return false;
    }

    if let Ok(version_output) = Command::new("valgrind").arg("--version").output() {
        if !version_output.status.success() {
            return false;
        }

        let version = String::from_utf8_lossy(&version_output.stdout);
        version.contains(VALGRIND_CODSPEED_VERSION)
    } else {
        false
    }
}

async fn install_valgrind(system_info: &SystemInfo) -> Result<()> {
    if is_valgrind_installed() {
        debug!("Valgrind is already installed with the correct version, skipping installation");
        return Ok(());
    }
    debug!("Installing valgrind");
    let valgrind_deb_url = format!(
        "https://github.com/CodSpeedHQ/valgrind-codspeed/releases/download/{}/{}",
        VALGRIND_CODSPEED_DEB_VERSION.as_str(),
        get_codspeed_valgrind_filename(system_info)?
    );
    let deb_path = env::temp_dir().join("valgrind-codspeed.deb");
    download_file(&Url::parse(valgrind_deb_url.as_str()).unwrap(), &deb_path).await?;

    run_with_sudo(&["apt-get", "update"])?;
    run_with_sudo(&[
        "apt-get",
        "install",
        "--allow-downgrades",
        "-y",
        deb_path.to_str().unwrap(),
    ])?;

    Ok(())
}

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
        info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Failed to install mongo-tracer");
    }

    Ok(())
}

pub async fn setup(system_info: &SystemInfo, config: &Config) -> Result<()> {
    install_valgrind(system_info).await?;

    // TODO: move into setup of the Instruments struct
    if config.instruments.is_mongodb_enabled() {
        install_mongodb_tracer().await?;
    }

    info!("Environment ready");
    Ok(())
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn test_system_info_to_codspeed_valgrind_version_ubuntu() {
        let system_info = SystemInfo {
            os: "ubuntu".to_string(),
            os_version: "22.04".to_string(),
            arch: "x86_64".to_string(),
            ..SystemInfo::test()
        };
        assert_snapshot!(
            get_codspeed_valgrind_filename(&system_info).unwrap(),
            @"valgrind_3.24.0-0codspeed1_ubuntu-22.04_amd64.deb"
        );
    }

    #[test]
    fn test_system_info_to_codspeed_valgrind_version_ubuntu_24() {
        let system_info = SystemInfo {
            os: "ubuntu".to_string(),
            os_version: "24.04".to_string(),
            arch: "x86_64".to_string(),
            ..SystemInfo::test()
        };
        assert_snapshot!(
            get_codspeed_valgrind_filename(&system_info).unwrap(),
            @"valgrind_3.24.0-0codspeed1_ubuntu-24.04_amd64.deb"
        );
    }

    #[test]
    fn test_system_info_to_codspeed_valgrind_version_debian() {
        let system_info = SystemInfo {
            os: "debian".to_string(),
            os_version: "12".to_string(),
            arch: "x86_64".to_string(),
            ..SystemInfo::test()
        };
        assert_snapshot!(
            get_codspeed_valgrind_filename(&system_info).unwrap(),
            @"valgrind_3.24.0-0codspeed1_ubuntu-22.04_amd64.deb"
        );
    }

    #[test]
    fn test_system_info_to_codspeed_valgrind_version_ubuntu_arm() {
        let system_info = SystemInfo {
            os: "ubuntu".to_string(),
            os_version: "22.04".to_string(),
            arch: "aarch64".to_string(),
            ..SystemInfo::test()
        };
        assert_snapshot!(
            get_codspeed_valgrind_filename(&system_info).unwrap(),
            @"valgrind_3.24.0-0codspeed1_ubuntu-22.04_arm64.deb"
        );
    }
}
