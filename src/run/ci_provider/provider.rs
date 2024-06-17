use git2::Repository;
use simplelog::SharedLogger;

use crate::prelude::*;
use crate::run::config::Config;
use crate::run::uploader::{Runner, UploadMetadata};

use super::interfaces::ProviderMetadata;

pub trait CIProviderDetector {
    /// Detects if the current environment is running inside the CI provider.
    fn detect() -> bool;
}

fn get_commit_hash(repository_root_path: &str) -> Result<String> {
    let repo = Repository::open(repository_root_path).context(format!(
        "Failed to open repository at path: {}",
        repository_root_path
    ))?;

    let commit_hash = repo
        .revparse_single("HEAD")
        .context("Failed to get HEAD commit")?
        .id()
        .to_string();
    Ok(commit_hash)
}

/// `CIProvider` is a trait that defines the necessary methods for a continuous integration provider.
pub trait CIProvider {
    /// Returns the logger for the CI provider.
    fn get_logger(&self) -> Box<dyn SharedLogger>;

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
    /// * `instruments` - A reference to the active instruments.
    ///
    /// # Example
    ///
    /// ```
    /// let provider = MyCIProvider::new();
    /// let config = Config::new();
    /// let instruments = Instruments::new();
    /// let metadata = provider.get_upload_metadata(&config, "abc123").unwrap();
    /// ```
    fn get_upload_metadata(&self, config: &Config, archive_hash: &str) -> Result<UploadMetadata> {
        let provider_metadata = self.get_provider_metadata()?;

        let commit_hash = get_commit_hash(&provider_metadata.repository_root_path)?;

        Ok(UploadMetadata {
            version: Some(2),
            tokenless: config.token.is_none(),
            provider_metadata,
            profile_md5: archive_hash.into(),
            commit_hash,
            runner: Runner {
                name: "codspeed-runner".into(),
                version: crate::VERSION.into(),
                instruments: config.instruments.get_active_instrument_names(),
            },
            platform: self.get_provider_slug().into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_commit_hash() {
        let commit_hash = get_commit_hash(env!("CARGO_MANIFEST_DIR")).unwrap();
        // ensure that the commit hash is correct, thus it has 40 characters
        assert_eq!(commit_hash.len(), 40);
    }
}
