use crate::run::runner::helpers::apt;
use crate::{VALGRIND_CODSPEED_DEB_VERSION, run::check_system::SystemInfo};
use crate::{
    VALGRIND_CODSPEED_VERSION, VALGRIND_CODSPEED_VERSION_STRING, prelude::*,
    run::helpers::download_file,
};
use semver::Version;
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

/// Parse a valgrind version string and extract the semantic version.
/// Expected format: "valgrind-3.25.1.codspeed" or "3.25.1.codspeed"
/// Returns Some(Version) if parsing succeeds, None otherwise.
fn parse_valgrind_codspeed_version(version_str: &str) -> Option<Version> {
    let version_str = version_str.trim();

    // Extract the version numbers before .codspeed
    let version_part = if let Some(codspeed_idx) = version_str.find(".codspeed") {
        &version_str[..codspeed_idx]
    } else {
        return None;
    };

    // Remove "valgrind-" prefix if present
    let version_part = version_part
        .strip_prefix("valgrind-")
        .unwrap_or(version_part);

    // Parse using semver
    Version::parse(version_part).ok()
}

fn is_valgrind_installed() -> bool {
    let is_valgrind_installed = Command::new("which")
        .arg("valgrind")
        .output()
        .is_ok_and(|output| output.status.success());
    if !is_valgrind_installed {
        debug!("valgrind is not installed");
        return false;
    }

    let Ok(version_output) = Command::new("valgrind").arg("--version").output() else {
        return false;
    };

    if !version_output.status.success() {
        debug!(
            "Failed to get valgrind version. stderr: {}",
            String::from_utf8_lossy(&version_output.stderr)
        );
        return false;
    }

    let version = String::from_utf8_lossy(&version_output.stdout);

    // Check if it's a codspeed version
    if !version.contains(".codspeed") {
        warn!(
            "Valgrind is installed but is not a CodSpeed version. expecting {} but found installed: {}",
            VALGRIND_CODSPEED_VERSION_STRING.as_str(),
            version.trim()
        );
        return false;
    }

    // Parse the installed version
    let Some(installed_version) = parse_valgrind_codspeed_version(&version) else {
        warn!(
            "Could not parse valgrind version. expecting {} but found installed: {}",
            VALGRIND_CODSPEED_VERSION_STRING.as_str(),
            version.trim()
        );
        return false;
    };
    if installed_version < VALGRIND_CODSPEED_VERSION {
        warn!(
            "Valgrind is installed but the version is too old. expecting {} or higher but found installed: {}",
            VALGRIND_CODSPEED_VERSION_STRING.as_str(),
            version.trim()
        );
        return false;
    }
    if installed_version > VALGRIND_CODSPEED_VERSION {
        warn!(
            "Using experimental valgrind version {}.codspeed. The recommended version is {}",
            installed_version,
            VALGRIND_CODSPEED_VERSION_STRING.as_str()
        );
    }
    true
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

    #[test]
    fn test_parse_valgrind_codspeed_version_with_prefix() {
        let version = parse_valgrind_codspeed_version("valgrind-3.25.1.codspeed").unwrap();
        assert_eq!(version, Version::new(3, 25, 1));
    }

    #[test]
    fn test_parse_valgrind_codspeed_version_without_prefix() {
        let version = parse_valgrind_codspeed_version("3.25.1.codspeed").unwrap();
        assert_eq!(version, Version::new(3, 25, 1));
    }

    #[test]
    fn test_parse_valgrind_codspeed_version_higher_patch() {
        let version = parse_valgrind_codspeed_version("valgrind-3.25.2.codspeed").unwrap();
        assert_eq!(version, Version::new(3, 25, 2));
    }

    #[test]
    fn test_parse_valgrind_codspeed_version_with_newline() {
        let version = parse_valgrind_codspeed_version("valgrind-3.25.1.codspeed\n").unwrap();
        assert_eq!(version, Version::new(3, 25, 1));
    }

    #[test]
    fn test_parse_valgrind_codspeed_version_without_codspeed_suffix() {
        assert_eq!(parse_valgrind_codspeed_version("valgrind-3.25.1"), None);
    }

    #[test]
    fn test_parse_valgrind_codspeed_version_invalid_format() {
        assert_eq!(
            parse_valgrind_codspeed_version("valgrind-3.25.codspeed"),
            None
        );
    }
}
