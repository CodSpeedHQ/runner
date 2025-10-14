use crate::run::runner::helpers::apt;
use crate::{VALGRIND_CODSPEED_DEB_VERSION, run::check_system::SystemInfo};
use crate::{VALGRIND_CODSPEED_VERSION, prelude::*, run::helpers::download_file};
use std::{env, path::Path, process::Command};
use url::Url;

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
        let result = version.contains(VALGRIND_CODSPEED_VERSION.as_str());
        if !result {
            warn!(
                "Valgrind is installed but the version is not the expected one. expecting {} but found installed: {}",
                VALGRIND_CODSPEED_VERSION.as_str(),
                version
            );
        }
        result
    } else {
        false
    }
}

pub async fn install_valgrind(
    system_info: &SystemInfo,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    apt::install_cached(
        system_info,
        setup_cache_dir,
        is_valgrind_installed,
        || async {
            debug!("Installing valgrind");
            let valgrind_deb_url = format!(
                "https://github.com/CodSpeedHQ/valgrind-codspeed/releases/download/{}/{}",
                VALGRIND_CODSPEED_DEB_VERSION.as_str(),
                get_codspeed_valgrind_filename(system_info)?
            );
            let deb_path = env::temp_dir().join("valgrind-codspeed.deb");
            download_file(&Url::parse(valgrind_deb_url.as_str()).unwrap(), &deb_path).await?;
            apt::install(system_info, &[deb_path.to_str().unwrap()])?;

            // Return package names for caching
            Ok(vec!["valgrind".to_string()])
        },
    )
    .await?;

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
