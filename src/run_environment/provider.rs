use async_trait::async_trait;
use git2::Repository;
use simplelog::SharedLogger;

use crate::api_client::CodSpeedAPIClient;
use crate::executor::{Config, ExecutorName};
use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::run::run_index_state::RunIndexState;
use crate::run::uploader::{
    LATEST_UPLOAD_METADATA_VERSION, ProfileArchive, Runner, UploadMetadata,
};

use super::interfaces::{RepositoryProvider, RunEnvironment, RunEnvironmentMetadata, RunPart};

pub trait RunEnvironmentDetector {
    /// Detects if the runner is currently executed within this run environment.
    fn detect() -> bool;
}

/// Audience to be used when requesting OIDC tokens.
///
/// It will be validated when the token is used to authenticate with CodSpeed.
///
/// This value must match the audience configured in CodSpeed backend.
static OIDC_AUDIENCE: &str = "codspeed.io";

/// `RunEnvironmentProvider` is a trait that defines the necessary methods
/// for a continuous integration provider.
#[async_trait(?Send)]
pub trait RunEnvironmentProvider {
    /// Returns the logger for the RunEnvironment.
    fn get_logger(&self) -> Box<dyn SharedLogger>;

    /// Returns the repository provider for this RunEnvironment
    fn get_repository_provider(&self) -> RepositoryProvider;

    /// Returns the run environment of the current provider.
    fn get_run_environment(&self) -> RunEnvironment;

    /// Returns the metadata related to the RunEnvironment.
    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata>;

    /// Return the metadata necessary to identify the `RunPart`
    fn get_run_provider_run_part(&self) -> Option<RunPart>;

    /// Get the OIDC audience that must be used when requesting OIDC tokens.
    ///
    /// It will be validated when the token is used to authenticate with CodSpeed.
    fn get_oidc_audience(&self) -> &str {
        OIDC_AUDIENCE
    }

    /// Check the OIDC configuration for the current run environment, if supported.
    fn check_oidc_configuration(&mut self, _config: &Config) -> Result<()> {
        Ok(())
    }

    /// Handle an OIDC token for the current run environment, if supported.
    ///
    /// Updates the config if necessary.
    ///
    /// Depending on the provider, this may involve requesting the token,
    /// warning the user about potential misconfigurations, or other necessary steps.
    ///
    /// Warning: OIDC tokens are typically short-lived. This method must be called
    /// just before the upload step to ensure the token is valid during the upload.
    async fn set_oidc_token(&self, _config: &mut Config) -> Result<()> {
        Ok(())
    }

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
    /// let metadata = provider.get_upload_metadata(&config, "abc123").await.unwrap();
    /// ```
    async fn get_upload_metadata(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        profile_archive: &ProfileArchive,
        executor_name: ExecutorName,
        _api_client: &CodSpeedAPIClient,
    ) -> Result<UploadMetadata> {
        let run_environment_metadata = self.get_run_environment_metadata()?;

        let commit_hash = self.get_commit_hash(&run_environment_metadata.repository_root_path)?;

        // Apply run index suffix to run_part if applicable.
        // This differentiates multiple uploads within the same CI job execution
        // (e.g., running both simulation and memory benchmarks in the same job).
        let run_part = self.get_run_provider_run_part().map(|run_part| {
            let run_index_state = RunIndexState::new(
                &run_environment_metadata.repository_root_path,
                &run_part.run_id,
                &run_part.run_part_id,
            );
            match run_index_state.get_and_increment() {
                Ok(run_index) => run_part.with_run_index(run_index),
                Err(e) => {
                    warn!("Failed to track run index: {e}. Continuing with index 0.");
                    run_part.with_run_index(0)
                }
            }
        });

        Ok(UploadMetadata {
            version: Some(LATEST_UPLOAD_METADATA_VERSION),
            tokenless: config.token.is_none(),
            repository_provider: self.get_repository_provider(),
            run_environment_metadata,
            profile_md5: profile_archive.hash.clone(),
            profile_encoding: profile_archive.content.encoding(),
            commit_hash,
            allow_empty: config.allow_empty,
            runner: Runner {
                name: "codspeed-runner".into(),
                version: crate::VERSION.into(),
                instruments: config.instruments.get_active_instrument_names(),
                executor: executor_name,
                system_info: system_info.clone(),
            },
            run_environment: self.get_run_environment(),
            run_part,
        })
    }

    /// Returns the HEAD commit hash of the repository at the given path.
    fn get_commit_hash(&self, repository_root_path: &str) -> Result<String> {
        get_commit_hash_default_impl(repository_root_path)
    }
}

fn get_commit_hash_default_impl(repository_root_path: &str) -> Result<String> {
    let repo = Repository::open(repository_root_path).context(format!(
        "Failed to open repository at path: {repository_root_path}"
    ))?;

    let commit_hash = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .context("Failed to get HEAD commit")?
        .id()
        .to_string();
    Ok(commit_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_commit_hash() {
        let commit_hash = get_commit_hash_default_impl(env!("CARGO_MANIFEST_DIR")).unwrap();
        // ensure that the commit hash is correct, thus it has 40 characters
        assert_eq!(commit_hash.len(), 40);
    }
}
