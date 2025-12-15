use crate::prelude::*;
use semver::Version;

/// Parse a version string from command output.
///
/// Expects the output format to be: "command_name version_string"
/// Example: "codspeed-memtrack 4.4.2"
pub(super) fn parse_from_output(output: &str) -> Result<Version> {
    let version_str = output
        .split_once(" ")
        .context("Unexpected version output format: missing space separator")?
        .1
        .trim();

    Version::parse(version_str)
        .with_context(|| format!("Failed to parse version from: {version_str}"))
}

/// Check if an installed version is compatible with the expected version.
///
/// Returns true if the installed version is greater than or equal to the expected version.
/// Logs warnings for outdated or experimental versions.
pub(super) fn is_compatible(command: &str, installed: &Version, expected: &Version) -> bool {
    match installed.cmp(expected) {
        std::cmp::Ordering::Less => {
            warn!(
                "{command} is installed but the version is too old. expecting {expected} or higher but found installed: {installed}",
            );
            false
        }
        std::cmp::Ordering::Greater => {
            warn!(
                "Using experimental {command} version {installed}. The recommended version is {expected}",
            );
            true
        }
        std::cmp::Ordering::Equal => true,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    mod parse_version_from_output {
        use super::*;

        #[test]
        fn parses_valid_version() {
            let output = "codspeed-memtrack 4.4.2";
            let version = parse_from_output(output).unwrap();
            assert_eq!(version, Version::new(4, 4, 2));
        }

        #[test]
        fn parses_version_with_prerelease() {
            let output = "codspeed-exec-harness 4.4.2-alpha.2";
            let version = parse_from_output(output).unwrap();
            assert_eq!(version.major, 4);
            assert_eq!(version.minor, 4);
            assert_eq!(version.patch, 2);
            assert_eq!(version.pre.as_str(), "alpha.2");
        }
    }

    mod is_version_compatible {
        use super::*;

        #[test]
        fn returns_true_for_equal_versions() {
            let installed = Version::new(4, 4, 2);
            let expected = Version::new(4, 4, 2);
            assert!(is_compatible("test-cmd", &installed, &expected));
        }

        #[test]
        fn returns_true_for_newer_version() {
            let installed = Version::new(4, 5, 0);
            let expected = Version::new(4, 4, 2);
            assert!(is_compatible("test-cmd", &installed, &expected));
        }

        #[test]
        fn returns_false_for_older_version() {
            let installed = Version::new(4, 3, 0);
            let expected = Version::new(4, 4, 2);
            assert!(!is_compatible("test-cmd", &installed, &expected));
        }

        #[test]
        fn handles_prerelease_versions() {
            let installed = Version::parse("4.4.2-alpha.2").unwrap();
            let expected = Version::new(4, 4, 1);
            // 4.4.2-alpha.2 > 4.4.1 because 4.4.2 > 4.4.1
            assert!(is_compatible("test-cmd", &installed, &expected));
        }

        #[test]
        fn prerelease_different_stage() {
            {
                let installed = Version::parse("4.4.2-alpha.2").unwrap();
                let expected = Version::new(4, 4, 2);
                // 4.4.2-alpha.2 < 4.4.2
                assert!(!is_compatible("test-cmd", &installed, &expected));
            }

            {
                let installed = Version::parse("4.4.2-beta.1").unwrap();
                let expected = Version::parse("4.4.2-alpha.1").unwrap();
                assert!(is_compatible("test-cmd", &installed, &expected));
            }

            {
                let installed = Version::new(4, 4, 2);
                let expected = Version::parse("4.4.2-alpha.2").unwrap();
                // 4.4.2 > 4.4.2-alpha.2
                assert!(is_compatible("test-cmd", &installed, &expected));
            }

            {
                let installed = Version::parse("4.4.2-alpha.1").unwrap();
                let expected = Version::parse("4.4.2-beta.1").unwrap();
                assert!(!is_compatible("test-cmd", &installed, &expected));
            }
        }

        #[test]
        fn prerelease_same_stage() {
            let installed = Version::parse("4.4.2-alpha.1").unwrap();
            let expected = Version::parse("4.4.2-alpha.2").unwrap();

            assert!(!is_compatible("test-cmd", &installed, &expected));
        }
    }
}
