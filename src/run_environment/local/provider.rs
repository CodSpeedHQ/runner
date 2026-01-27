use async_trait::async_trait;
use git2::Repository;
use simplelog::SharedLogger;

use crate::api_client::CodSpeedAPIClient;
use crate::cli::run::check_system::SystemInfo;
use crate::cli::run::helpers::{GitRemote, find_repository_root, parse_git_remote};
use crate::cli::run::uploader::{
    LATEST_UPLOAD_METADATA_VERSION, ProfileArchive, Runner, UploadMetadata,
};
use crate::executor::config::RepositoryOverride;
use crate::executor::{Config, ExecutorName};
use crate::local_logger::get_local_logger;
use crate::prelude::*;
use crate::run_environment::interfaces::{RepositoryProvider, RunEnvironmentMetadata, RunEvent};
use crate::run_environment::provider::{RunEnvironmentDetector, RunEnvironmentProvider};
use crate::run_environment::{RunEnvironment, RunPart};

static FAKE_COMMIT_REF: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Debug)]
enum RepositorySource {
    /// We have repository information (from git or from override)
    Git {
        repository_provider: RepositoryProvider,
        owner: String,
        repository: String,
        ref_: String,
        head_ref: Option<String>,
    },
    /// Not in a git repo, no override - fetch repository info from API using project name
    ApiProject { project_name: String },
}

#[derive(Debug)]
pub struct LocalProvider {
    source: RepositorySource,
    pub event: RunEvent,
    pub repository_root_path: String,
}

impl TryFrom<&Config> for LocalProvider {
    type Error = Error;
    fn try_from(config: &Config) -> Result<Self> {
        let current_dir = std::env::current_dir()?;

        let repository_root_path = {
            let Some(mut path) = find_repository_root(&current_dir) else {
                // We are not in a git repository
                if let Some(RepositoryOverride {
                    owner,
                    repository,
                    repository_provider,
                }) = config.repository_override.clone()
                {
                    // Use the repository_override with very minimal information
                    return Ok(Self {
                        source: RepositorySource::Git {
                            repository_provider,
                            ref_: FAKE_COMMIT_REF.to_string(),
                            head_ref: None,
                            owner,
                            repository,
                        },
                        repository_root_path: current_dir.to_string_lossy().to_string(),
                        event: RunEvent::Local,
                    });
                } else {
                    // No git repo and no override - we'll fetch from API using default project name
                    return Ok(Self {
                        source: RepositorySource::ApiProject {
                            project_name: crate::cli::exec::DEFAULT_REPOSITORY_NAME.to_string(),
                        },
                        repository_root_path: current_dir.to_string_lossy().to_string(),
                        event: RunEvent::Local,
                    });
                }
            };

            // Add a trailing slash to the path
            path.push("");
            path.to_string_lossy().to_string()
        };

        let git_repository = Repository::open(repository_root_path.clone()).context(format!(
            "Failed to open repository at path: {repository_root_path}"
        ))?;

        let remote = git_repository.find_remote("origin")?;

        let (repository_provider, owner, repository) =
            if let Some(repo_override) = config.repository_override.clone() {
                (
                    repo_override.repository_provider,
                    repo_override.owner,
                    repo_override.repository,
                )
            } else {
                extract_provider_owner_and_repository_from_remote_url(remote.url().unwrap())?
            };

        let head = git_repository.head().context("Failed to get HEAD")?;
        let ref_ = head
            .peel_to_commit()
            .context("Failed to get HEAD commit")?
            .id()
            .to_string();
        let head_ref = if head.is_branch() {
            let branch = head.shorthand().context("Failed to get HEAD branch name")?;
            Some(branch.to_string())
        } else {
            None
        };

        Ok(Self {
            source: RepositorySource::Git {
                repository_provider,
                ref_,
                head_ref,
                owner,
                repository,
            },
            event: RunEvent::Local,
            repository_root_path,
        })
    }
}

impl RunEnvironmentDetector for LocalProvider {
    fn detect() -> bool {
        true
    }
}

#[async_trait(?Send)]
impl RunEnvironmentProvider for LocalProvider {
    fn get_repository_provider(&self) -> RepositoryProvider {
        match &self.source {
            RepositorySource::Git {
                repository_provider,
                ..
            } => repository_provider.clone(),
            RepositorySource::ApiProject { .. } => {
                // Placeholder, will be updated from API
                RepositoryProvider::GitHub
            }
        }
    }

    fn get_logger(&self) -> Box<dyn SharedLogger> {
        get_local_logger()
    }

    fn get_run_environment(&self) -> RunEnvironment {
        RunEnvironment::Local
    }

    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata> {
        match &self.source {
            RepositorySource::Git {
                owner,
                repository,
                ref_,
                head_ref,
                ..
            } => Ok(RunEnvironmentMetadata {
                base_ref: None,
                head_ref: head_ref.clone(),
                event: self.event.clone(),
                gh_data: None,
                gl_data: None,
                sender: None,
                owner: owner.clone(),
                repository: repository.clone(),
                ref_: ref_.clone(),
                repository_root_path: self.repository_root_path.clone(),
            }),
            RepositorySource::ApiProject { .. } => Ok(RunEnvironmentMetadata {
                base_ref: None,
                head_ref: None,
                event: self.event.clone(),
                gh_data: None,
                gl_data: None,
                sender: None,
                owner: String::new(),
                repository: String::new(),
                ref_: FAKE_COMMIT_REF.to_string(),
                repository_root_path: self.repository_root_path.clone(),
            }),
        }
    }

    async fn get_upload_metadata(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        profile_archive: &ProfileArchive,
        executor_name: ExecutorName,
        api_client: &CodSpeedAPIClient,
    ) -> Result<UploadMetadata> {
        let mut run_environment_metadata = self.get_run_environment_metadata()?;
        let mut repository_provider = self.get_repository_provider();

        // If we need to fetch repository info from the API
        if let RepositorySource::ApiProject { project_name } = &self.source {
            debug!("Fetching repository info from API for project: {project_name}");
            let repo_info = api_client
                .get_or_create_project_repository(
                    crate::api_client::GetOrCreateProjectRepositoryVars {
                        name: project_name.clone(),
                    },
                )
                .await?;

            debug!("Received repository info: {repo_info:?}");

            // Update the metadata with the fetched values
            run_environment_metadata.owner = repo_info.owner;
            run_environment_metadata.repository = repo_info.name;

            repository_provider = repo_info.provider;
        }

        Ok(UploadMetadata {
            version: Some(LATEST_UPLOAD_METADATA_VERSION),
            tokenless: config.token.is_none(),
            repository_provider,
            commit_hash: run_environment_metadata.ref_.clone(),
            allow_empty: config.allow_empty,
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

    /// For local runs have, we cannot really send anything here
    fn get_run_provider_run_part(&self) -> Option<RunPart> {
        None
    }
}

fn extract_provider_owner_and_repository_from_remote_url(
    remote_url: &str,
) -> Result<(RepositoryProvider, String, String)> {
    let GitRemote {
        domain,
        owner,
        repository,
    } = parse_git_remote(remote_url)?;
    let repository_provider = match domain.as_str() {
        "github.com" => RepositoryProvider::GitHub,
        "gitlab.com" => RepositoryProvider::GitLab,
        domain => bail!("Repository provider {domain} is not supported by CodSpeed"),
    };

    Ok((
        repository_provider,
        owner.to_string(),
        repository.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    // use crate::VERSION;
    // use insta::assert_json_snapshot;

    use super::*;

    #[test]
    fn test_extract_provider_owner_and_repository_from_remote_url() {
        let remote_urls = [
            (
                "git@github.com:CodSpeedHQ/codspeed.git",
                RepositoryProvider::GitHub,
                "CodSpeedHQ",
                "codspeed",
            ),
            (
                "https://github.com/CodSpeedHQ/codspeed.git",
                RepositoryProvider::GitHub,
                "CodSpeedHQ",
                "codspeed",
            ),
            (
                "git@gitlab.com:codspeed/runner.git",
                RepositoryProvider::GitLab,
                "codspeed",
                "runner",
            ),
            (
                "https://gitlab.com/codspeed/runner.git",
                RepositoryProvider::GitLab,
                "codspeed",
                "runner",
            ),
        ];
        for (remote_url, expected_provider, expected_owner, expected_repository) in
            remote_urls.into_iter()
        {
            let (repository_provider, owner, repository) =
                extract_provider_owner_and_repository_from_remote_url(remote_url).unwrap();
            assert_eq!(repository_provider, expected_provider);
            assert_eq!(owner, expected_owner);
            assert_eq!(repository, expected_repository);
        }
    }

    #[test]
    fn fake_commit_hash_ref() {
        assert_eq!(FAKE_COMMIT_REF.len(), 40);
    }

    // TODO: uncomment later when we have a way to mock git repository
    // #[test]
    // fn test_provider_metadata() {
    //     let config = Config {
    //         token: Some("token".into()),
    //         ..Config::test()
    //     };
    //     let local_provider = LocalProvider::try_from(&config).unwrap();
    //     let provider_metadata = local_provider.get_provider_metadata().unwrap();

    //     assert_json_snapshot!(provider_metadata, {
    //         ".runner.version" => insta::dynamic_redaction(|value,_path| {
    //             assert_eq!(value.as_str().unwrap(), VERSION.to_string());
    //             "[version]"
    //         }),
    //     });
    // }
}
