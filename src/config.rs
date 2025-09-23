use std::{env, fs, path::PathBuf};

use crate::prelude::*;
use nestify::nest;
use serde::{Deserialize, Serialize};

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "kebab-case")]*
    pub struct CodSpeedConfig {
        pub auth: pub struct AuthConfig {
            pub token: Option<String>,
        }
    }
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
            auth: AuthConfig { token: None },
        }
    }
}

impl CodSpeedConfig {
    /// Load the configuration. If it does not exist, return a default configuration.
    ///
    /// If oauth_token_override is provided, the token from the loaded configuration will be
    /// ignored, and the override will be used instead
    pub fn load_with_override(oauth_token_override: Option<&str>) -> Result<Self> {
        let config_path = get_configuration_file_path();

        let mut config = match fs::read(&config_path) {
            Ok(config_str) => {
                let config = serde_yaml::from_slice(&config_str).context(format!(
                    "Failed to parse CodSpeed config at {}",
                    config_path.display()
                ))?;
                debug!("Config loaded from {}", config_path.display());
                config
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("Config file not found at {}", config_path.display());
                CodSpeedConfig::default()
            }
            Err(e) => bail!("Failed to load config: {e}"),
        };

        if let Some(oauth_token) = oauth_token_override {
            config.auth.token = Some(oauth_token.to_owned());
        }

        Ok(config)
    }

    /// Load the configuration. If it does not exist, return a default configuration.
    pub fn load() -> Result<Self> {
        Self::load_with_override(None)
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
