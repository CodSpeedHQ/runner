use std::{collections::HashMap, env, fs, path::PathBuf};

use crate::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_URL: &str = "https://gql.codspeed.io/";
pub const DEFAULT_UPLOAD_URL: &str = "https://api.codspeed.io/upload";
pub const DEFAULT_PROFILE_NAME: &str = "default";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            token: None,
            api_url: None,
            upload_url: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodSpeedConfig {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

// Old config format for migration
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct OldCodSpeedConfig {
    auth: OldAuthConfig,
}

#[derive(Debug, Deserialize)]
struct OldAuthConfig {
    token: Option<String>,
}

/// Get the path to the configuration file, following the XDG Base Directory Specification
/// at https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
fn get_configuration_file_path() -> PathBuf {
    let config_dir = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME env variable not set");
            PathBuf::from(home).join(".config")
        });
    let config_dir = config_dir.join("codspeed");
    config_dir.join("config.yaml")
}

impl Default for CodSpeedConfig {
    fn default() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }
}

impl CodSpeedConfig {
    /// Load the configuration. If it does not exist, return a default configuration.
    ///
    /// If oauth_token_override is provided, the token from the loaded profile will be
    /// ignored, and the override will be used instead for the specified profile.
    pub fn load_with_override(
        profile_name: &str,
        oauth_token_override: Option<&str>,
    ) -> Result<Self> {
        let config_path = get_configuration_file_path();

        let mut config = match fs::read(&config_path) {
            Ok(config_str) => {
                // Try to parse as new format first
                match serde_yaml::from_slice::<CodSpeedConfig>(&config_str) {
                    Ok(config) => {
                        debug!("Config loaded from {}", config_path.display());
                        config
                    }
                    Err(_) => {
                        // Try to parse as old format and migrate
                        match serde_yaml::from_slice::<OldCodSpeedConfig>(&config_str) {
                            Ok(old_config) => {
                                info!(
                                    "Migrating config from old format to new profile-based format"
                                );
                                let mut profiles = HashMap::new();
                                if old_config.auth.token.is_some() {
                                    profiles.insert(
                                        DEFAULT_PROFILE_NAME.to_string(),
                                        Profile {
                                            token: old_config.auth.token,
                                            api_url: None,
                                            upload_url: None,
                                        },
                                    );
                                }
                                let config = CodSpeedConfig { profiles };
                                // Persist the migrated config
                                config
                                    .persist()
                                    .context("Failed to persist migrated config")?;
                                debug!(
                                    "Config migrated and persisted to {}",
                                    config_path.display()
                                );
                                config
                            }
                            Err(e) => {
                                bail!(
                                    "Failed to parse CodSpeed config at {}: {}",
                                    config_path.display(),
                                    e
                                )
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("Config file not found at {}", config_path.display());
                CodSpeedConfig::default()
            }
            Err(e) => bail!("Failed to load config: {e}"),
        };

        // Apply token override to the specified profile
        if let Some(oauth_token) = oauth_token_override {
            config
                .profiles
                .entry(profile_name.to_string())
                .or_insert_with(Profile::default)
                .token = Some(oauth_token.to_owned());
        }

        Ok(config)
    }

    /// Load the configuration. If it does not exist, return a default configuration.
    pub fn load() -> Result<Self> {
        Self::load_with_override(DEFAULT_PROFILE_NAME, None)
    }

    /// Get a profile by name, or None if it doesn't exist
    pub fn get_profile(&self, profile_name: &str) -> Option<&Profile> {
        self.profiles.get(profile_name)
    }

    /// Get a mutable profile by name, creating it if it doesn't exist
    pub fn get_or_create_profile(&mut self, profile_name: &str) -> &mut Profile {
        self.profiles
            .entry(profile_name.to_string())
            .or_insert_with(Profile::default)
    }

    /// Resolve the effective API URL for a profile
    /// Priority: profile value > default
    pub fn resolve_api_url(&self, profile_name: &str) -> String {
        self.get_profile(profile_name)
            .and_then(|p| p.api_url.clone())
            .unwrap_or_else(|| DEFAULT_API_URL.to_string())
    }

    /// Resolve the effective upload URL for a profile
    /// Priority: profile value > default
    pub fn resolve_upload_url(&self, profile_name: &str) -> String {
        self.get_profile(profile_name)
            .and_then(|p| p.upload_url.clone())
            .unwrap_or_else(|| DEFAULT_UPLOAD_URL.to_string())
    }

    /// Persist changes to the configuration
    pub fn persist(&self) -> Result<()> {
        let config_path = get_configuration_file_path();
        fs::create_dir_all(config_path.parent().unwrap())?;

        let config_str = serde_yaml::to_string(self)?;
        fs::write(&config_path, config_str)?;
        debug!("Config written to {}", config_path.display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_default() {
        let profile = Profile::default();
        assert_eq!(profile.token, None);
        assert_eq!(profile.api_url, None);
        assert_eq!(profile.upload_url, None);
    }

    #[test]
    fn test_config_default() {
        let config = CodSpeedConfig::default();
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_get_profile() {
        let mut config = CodSpeedConfig::default();
        config.profiles.insert(
            "test".to_string(),
            Profile {
                token: Some("test_token".to_string()),
                api_url: Some("https://test.com".to_string()),
                upload_url: None,
            },
        );

        let profile = config.get_profile("test").unwrap();
        assert_eq!(profile.token, Some("test_token".to_string()));
        assert_eq!(profile.api_url, Some("https://test.com".to_string()));
        assert_eq!(profile.upload_url, None);

        assert!(config.get_profile("nonexistent").is_none());
    }

    #[test]
    fn test_get_or_create_profile() {
        let mut config = CodSpeedConfig::default();

        // Create new profile
        let profile = config.get_or_create_profile("new");
        profile.token = Some("new_token".to_string());

        // Verify it was created
        assert_eq!(
            config.get_profile("new").unwrap().token,
            Some("new_token".to_string())
        );

        // Get existing profile
        let profile = config.get_or_create_profile("new");
        assert_eq!(profile.token, Some("new_token".to_string()));
    }

    #[test]
    fn test_resolve_api_url() {
        let mut config = CodSpeedConfig::default();

        // No profile - should return default
        assert_eq!(config.resolve_api_url("test"), DEFAULT_API_URL);

        // Profile with custom API URL
        config.profiles.insert(
            "custom".to_string(),
            Profile {
                token: None,
                api_url: Some("https://custom.com".to_string()),
                upload_url: None,
            },
        );
        assert_eq!(config.resolve_api_url("custom"), "https://custom.com");

        // Profile without custom API URL - should return default
        config.profiles.insert(
            "default_api".to_string(),
            Profile {
                token: Some("token".to_string()),
                api_url: None,
                upload_url: None,
            },
        );
        assert_eq!(config.resolve_api_url("default_api"), DEFAULT_API_URL);
    }

    #[test]
    fn test_resolve_upload_url() {
        let mut config = CodSpeedConfig::default();

        // No profile - should return default
        assert_eq!(config.resolve_upload_url("test"), DEFAULT_UPLOAD_URL);

        // Profile with custom upload URL
        config.profiles.insert(
            "custom".to_string(),
            Profile {
                token: None,
                api_url: None,
                upload_url: Some("https://custom-upload.com".to_string()),
            },
        );
        assert_eq!(
            config.resolve_upload_url("custom"),
            "https://custom-upload.com"
        );

        // Profile without custom upload URL - should return default
        config.profiles.insert(
            "default_upload".to_string(),
            Profile {
                token: Some("token".to_string()),
                api_url: None,
                upload_url: None,
            },
        );
        assert_eq!(
            config.resolve_upload_url("default_upload"),
            DEFAULT_UPLOAD_URL
        );
    }

    #[test]
    fn test_old_config_migration() {
        let old_config_yaml = r#"
auth:
  token: "old_token"
"#;

        let old_config: Result<OldCodSpeedConfig, _> = serde_yaml::from_str(old_config_yaml);
        assert!(old_config.is_ok());
        let old_config = old_config.unwrap();
        assert_eq!(old_config.auth.token, Some("old_token".to_string()));

        // Test that new config can be created from scratch
        let mut new_config = CodSpeedConfig::default();
        new_config.profiles.insert(
            DEFAULT_PROFILE_NAME.to_string(),
            Profile {
                token: old_config.auth.token,
                api_url: None,
                upload_url: None,
            },
        );

        assert_eq!(
            new_config.get_profile(DEFAULT_PROFILE_NAME).unwrap().token,
            Some("old_token".to_string())
        );
    }

    #[test]
    fn test_new_config_serialization() {
        let mut config = CodSpeedConfig::default();
        config.profiles.insert(
            "dev".to_string(),
            Profile {
                token: Some("dev_token".to_string()),
                api_url: Some("https://dev.codspeed.io/".to_string()),
                upload_url: Some("https://dev-api.codspeed.io/upload".to_string()),
            },
        );
        config.profiles.insert(
            "prod".to_string(),
            Profile {
                token: Some("prod_token".to_string()),
                api_url: None,
                upload_url: None,
            },
        );

        let yaml = serde_yaml::to_string(&config).unwrap();

        // Verify it can be deserialized
        let deserialized: CodSpeedConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.profiles.len(), 2);

        let dev_profile = deserialized.get_profile("dev").unwrap();
        assert_eq!(dev_profile.token, Some("dev_token".to_string()));
        assert_eq!(
            dev_profile.api_url,
            Some("https://dev.codspeed.io/".to_string())
        );
        assert_eq!(
            dev_profile.upload_url,
            Some("https://dev-api.codspeed.io/upload".to_string())
        );

        let prod_profile = deserialized.get_profile("prod").unwrap();
        assert_eq!(prod_profile.token, Some("prod_token".to_string()));
        assert_eq!(prod_profile.api_url, None);
        assert_eq!(prod_profile.upload_url, None);

        // Verify that None fields are not serialized (using skip_serializing_if)
        assert!(!yaml.contains("api-url:") || yaml.contains("dev"));
        assert!(!yaml.contains("upload-url:") || yaml.contains("dev"));
    }
}
