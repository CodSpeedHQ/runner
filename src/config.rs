use std::{env, path::PathBuf};

use crate::prelude::*;
use nestify::nest;
use serde::{Deserialize, Serialize};

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "kebab-case")]*
    pub struct Config {
        pub auth: pub struct AuthConfig {
            pub token: String,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            auth: AuthConfig { token: "".into() },
        }
    }
}

impl Config {
    /// Load the configuration. If it does not exist, store and return a default configuration
    pub async fn load() -> Result<Self> {
        let config_path = get_configuration_file_path();

        match tokio::fs::read(&config_path).await {
            Ok(config_str) => {
                let config = serde_yaml::from_slice(&config_str).context(format!(
                    "Failed to parse CodSpeed config at {}",
                    config_path.display()
                ))?;
                debug!("Config loaded from {}", config_path.display());
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("Config file not found at {}", config_path.display());
                let config = Config::default();
                config.persist().await?;
                Ok(config)
            }
            Err(e) => bail!("Failed to load config: {}", e),
        }
    }

    /// Persist changes to the configuration
    pub async fn persist(&self) -> Result<()> {
        let config_path = get_configuration_file_path();
        tokio::fs::create_dir_all(config_path.parent().unwrap()).await?;

        let config_str = serde_yaml::to_string(self)?;
        tokio::fs::write(&config_path, config_str).await?;
        debug!("Config written to {}", config_path.display());

        Ok(())
    }
}
