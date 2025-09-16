use git2::Repository;
use simplelog::SharedLogger;

use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::run::config::Config;
use crate::run::runner::ExecutorName;
use crate::run::uploader::{Runner, UploadMetadata};

use super::interfaces::{RepositoryProvider, RunEnvironment, RunEnvironmentMetadata, RunPart};

pub trait RunEnvironmentDetector {
    /// Detects if the runner is currently executed within this run environment.
    fn detect() -> bool;
}

fn get_commit_hash(repository_root_path: &str) -> Result<String> {
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

/// `RunEnvironmentProvider` is a trait that defines the necessary methods
/// for a continuous integration provider.
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
    fn get_upload_metadata(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        archive_hash: &str,
        content_encoding: Option<String>,
        executor_name: ExecutorName,
    ) -> Result<UploadMetadata> {
        let run_environment_metadata = self.get_run_environment_metadata()?;

        let commit_hash = get_commit_hash(&run_environment_metadata.repository_root_path)?;

        Ok(UploadMetadata {
            version: Some(7),
            tokenless: config.token.is_none(),
            repository_provider: self.get_repository_provider(),
            run_environment_metadata,
            profile_md5: archive_hash.into(),
            content_encoding,
            commit_hash,
            runner: Runner {
                name: "codspeed-runner".into(),
                version: crate::VERSION.into(),
                instruments: config.instruments.get_active_instrument_names(),
                executor: executor_name,
                system_info: system_info.clone(),
            },
            run_environment: self.get_run_environment(),
            run_part: self.get_run_provider_run_part(),
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
