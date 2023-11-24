use crate::{
    ci_provider::github_actions_provider::GitHubActionsProvider, config::Config, prelude::*,
    uploader::UploadMetadata,
};

mod github_actions_provider;

/// `CIProvider` is a trait that defines the necessary methods for a continuous integration provider.
pub trait CIProvider {
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

    /// Detects if the current environment is running inside the CI provider.
    ///
    /// # Example
    ///
    /// ```
    /// assert_eq!(MyCIProvider::detect(), true);
    /// ```
    fn detect() -> bool;

    /// Returns the metadata necessary for uploading results to the CI provider.
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
    fn get_upload_metadata(&self, config: &Config, archive_hash: &str) -> Result<UploadMetadata>;
}

pub fn get_provider(config: &Config) -> Result<impl CIProvider> {
    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(provider);
    }

    bail!("No CI provider detected")
}
