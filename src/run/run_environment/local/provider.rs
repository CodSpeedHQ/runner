use git2::Repository;
use simplelog::SharedLogger;

use crate::local_logger::get_local_logger;
use crate::prelude::*;
use crate::run::helpers::{parse_git_remote, GitRemote};
use crate::run::run_environment::interfaces::RunEnvironment;
use crate::run::{
    config::Config,
    helpers::find_repository_root,
    run_environment::{
        interfaces::{RepositoryProvider, RunEnvironmentMetadata, RunEvent},
        provider::{RunEnvironmentDetector, RunEnvironmentProvider},
    },
};

#[derive(Debug)]
pub struct LocalProvider {
    repository_provider: RepositoryProvider,
    pub ref_: String,
    pub owner: String,
    pub repository: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub event: RunEvent,
    pub repository_root_path: String,
}

impl LocalProvider {}

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
        domain => bail!(
            "Repository provider {} is not supported by CodSpeed",
            domain
        ),
    };

    Ok((
        repository_provider,
        owner.to_string(),
        repository.to_string(),
    ))
}

impl TryFrom<&Config> for LocalProvider {
    type Error = Error;
    fn try_from(_config: &Config) -> Result<Self> {
        let repository_root_path = match find_repository_root(&std::env::current_dir()?) {
            Some(mut path) => {
                // Add a trailing slash to the path
                path.push("");
                path.to_string_lossy().to_string()
            },
            None => bail!("Could not find repository root, please make sure you are running the command from inside a git repository"),
        };

        let git_repository = Repository::open(repository_root_path.clone()).context(format!(
            "Failed to open repository at path: {}",
            repository_root_path
        ))?;

        let remote = git_repository.find_remote("origin")?;
        let (repository_provider, owner, repository) =
            extract_provider_owner_and_repository_from_remote_url(remote.url().unwrap())?;

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
            repository_provider,
            ref_,
            head_ref,
            base_ref: None,
            owner,
            repository,
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

impl RunEnvironmentProvider for LocalProvider {
    fn get_repository_provider(&self) -> RepositoryProvider {
        self.repository_provider.clone()
    }

    fn get_logger(&self) -> Box<dyn SharedLogger> {
        get_local_logger()
    }

    fn get_run_environment(&self) -> RunEnvironment {
        RunEnvironment::Local
    }

    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata> {
        Ok(RunEnvironmentMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gh_data: None,
            gl_data: None,
            sender: None,
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
        })
    }
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
                "git@github.com:CodSpeedHQ/runner.git",
                RepositoryProvider::GitHub,
                "CodSpeedHQ",
                "runner",
            ),
            (
                "https://github.com/CodSpeedHQ/runner.git",
                RepositoryProvider::GitHub,
                "CodSpeedHQ",
                "runner",
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
