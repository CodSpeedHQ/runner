use async_trait::async_trait;
use simplelog::SharedLogger;

use crate::executor::config::RepositoryOverride;
use crate::executor::{Config, ExecutorName};
use crate::local_logger::get_local_logger;
use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::run::uploader::{
    LATEST_UPLOAD_METADATA_VERSION, ProfileArchive, Runner, UploadMetadata,
};
use crate::run_environment::interfaces::{RepositoryProvider, RunEnvironmentMetadata, RunEvent};
use crate::run_environment::provider::{RunEnvironmentDetector, RunEnvironmentProvider};
use crate::run_environment::{RunEnvironment, RunPart};

static FAKE_COMMIT_REF: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Debug)]
pub struct ProjectProvider {
    repository_override: Option<RepositoryOverride>,
    pub ref_: String,
    pub event: RunEvent,
    pub repository_root_path: String,
}

impl TryFrom<&Config> for ProjectProvider {
    type Error = Error;
    fn try_from(config: &Config) -> Result<Self> {
        let current_dir = std::env::current_dir()?;

        Ok(Self {
            repository_override: config.repository_override.clone(),
            ref_: FAKE_COMMIT_REF.to_string(),
            repository_root_path: current_dir.to_string_lossy().to_string(),
            event: RunEvent::Project,
        })
    }
}

impl RunEnvironmentDetector for ProjectProvider {
    fn detect() -> bool {
        // Never auto-detect - must be explicitly chosen
        false
    }
}

#[async_trait(?Send)]
impl RunEnvironmentProvider for ProjectProvider {
    fn get_repository_provider(&self) -> RepositoryProvider {
        // This should not be called in normal exec flow
        // If it is called and there's no override, we return the default provider
        self.repository_override
            .as_ref()
            .map(|override_| override_.repository_provider.clone())
            .unwrap_or_default()
    }

    fn get_logger(&self) -> Box<dyn SharedLogger> {
        get_local_logger()
    }

    fn get_run_environment(&self) -> RunEnvironment {
        RunEnvironment::Project
    }

    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata> {
        let (owner, repository) = if let Some(override_) = &self.repository_override {
            (override_.owner.clone(), override_.repository.clone())
        } else {
            bail!("Could not get repository information - no repository override provided");
        };

        Ok(RunEnvironmentMetadata {
            base_ref: None,
            head_ref: None,
            event: self.event.clone(),
            gh_data: None,
            gl_data: None,
            sender: None,
            owner,
            repository,
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
        })
    }

    fn get_upload_metadata(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        profile_archive: &ProfileArchive,
        executor_name: ExecutorName,
    ) -> Result<UploadMetadata> {
        let run_environment_metadata = self.get_run_environment_metadata()?;

        Ok(UploadMetadata {
            version: Some(LATEST_UPLOAD_METADATA_VERSION),
            tokenless: config.token.is_none(),
            repository_provider: self.get_repository_provider(),
            commit_hash: run_environment_metadata.ref_.clone(),
            run_environment_metadata,
            profile_md5: profile_archive.hash.clone(),
            profile_encoding: profile_archive.content.encoding(),
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

    /// For project runs, we cannot send anything here (no CI environment)
    fn get_run_provider_run_part(&self) -> Option<RunPart> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_commit_hash_ref() {
        assert_eq!(FAKE_COMMIT_REF.len(), 40);
    }

    #[test]
    fn test_project_provider_never_detects() {
        assert!(!ProjectProvider::detect());
    }
}
