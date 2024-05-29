use git2::Repository;
use lazy_static::lazy_static;
use simplelog::SharedLogger;

use crate::logger::get_local_logger;
use crate::prelude::*;
use crate::run::{
    ci_provider::{
        interfaces::{ProviderMetadata, RunEvent},
        provider::{CIProvider, CIProviderDetector},
    },
    config::Config,
    helpers::find_repository_root,
};

#[derive(Debug)]
pub struct LocalProvider {
    pub ref_: String,
    pub owner: String,
    pub repository: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub event: RunEvent,
    pub repository_root_path: String,
}

impl LocalProvider {}

lazy_static! {
    static ref REMOTE_REGEX: regex::Regex =
        regex::Regex::new(r"[:/](?P<owner>[^/]+)/(?P<repository>[^/]+)\.git").unwrap();
}

fn extract_owner_and_repository_from_remote_url(remote_url: &str) -> Result<(String, String)> {
    let captures = REMOTE_REGEX.captures(remote_url).ok_or_else(|| {
        anyhow!(
            "Could not extract owner and repository from remote url: {}",
            remote_url
        )
    })?;

    let owner = captures.name("owner").unwrap().as_str();
    let repository = captures.name("repository").unwrap().as_str();

    Ok((owner.to_string(), repository.to_string()))
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
        let (owner, repository) =
            extract_owner_and_repository_from_remote_url(remote.url().unwrap())?;

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

impl CIProviderDetector for LocalProvider {
    fn detect() -> bool {
        true
    }
}

impl CIProvider for LocalProvider {
    fn get_logger(&self) -> Box<dyn SharedLogger> {
        get_local_logger()
    }

    fn get_provider_name(&self) -> &'static str {
        "Local"
    }

    fn get_provider_slug(&self) -> &'static str {
        "local"
    }

    fn get_provider_metadata(&self) -> Result<ProviderMetadata> {
        Ok(ProviderMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gh_data: None,
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
    fn test_extract_owner_and_repository_from_remote_url() {
        let remote_urls = [
            "git@github.com:CodSpeedHQ/runner.git",
            "https://github.com/CodSpeedHQ/runner.git",
        ];
        for remote_url in remote_urls.iter() {
            let (owner, repository) =
                extract_owner_and_repository_from_remote_url(remote_url).unwrap();
            assert_eq!(owner, "CodSpeedHQ");
            assert_eq!(repository, "runner");
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
