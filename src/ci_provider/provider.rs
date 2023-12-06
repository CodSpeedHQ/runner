use crate::config::Config;
use crate::prelude::*;
use crate::uploader::{Runner, UploadMetadata};

use super::interfaces::ProviderMetadata;

pub trait CIProviderDetector {
    /// Detects if the current environment is running inside the CI provider.
    fn detect() -> bool;
}

/// `CIProvider` is a trait that defines the necessary methods for a continuous integration provider.
pub trait CIProvider {
    /// Registers the logger for the CI provider.
    fn setup_logger(&self) -> Result<()>;

    /// Returns the name of the CI provider.
    ///
    /// # Example
    ///
    /// ```
    /// let provider = MyCIProvider::new();
    /// assert_eq!(provider.get_provider_name(), "MyCIProvider");
    /// ```
    fn get_provider_name(&self) -> &'static str;

    /// Returns the slug of the CI provider.
    ///
    /// # Example
    ///
    /// ```
    /// let provider = MyCIProvider::new();
    /// assert_eq!(provider.get_provider_slug(), "my-ci-provider");
    /// ```
    fn get_provider_slug(&self) -> &'static str;

    /// Returns the metadata related to the CI provider.
    fn get_provider_metadata(&self) -> Result<ProviderMetadata>;

    /// Returns the metadata necessary for uploading results to CodSpeed.
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the configuration.
    /// * `archive_hash` - The hash of the archive to be uploaded.
    ///
    /// # Example
    ///
    /// ```
    /// let provider = MyCIProvider::new();
    /// let config = Config::new();
    /// let metadata = provider.get_upload_metadata(&config, "abc123").unwrap();
    /// ```
    fn get_upload_metadata(&self, config: &Config, archive_hash: &str) -> Result<UploadMetadata> {
        let provider_metadata = self.get_provider_metadata()?;

        Ok(UploadMetadata {
            version: Some(1),
            tokenless: config.token.is_none(),
            provider_metadata,
            profile_md5: archive_hash.into(),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: crate::VERSION.into(),
            },
            platform: self.get_provider_slug().into(),
        })
    }
}
