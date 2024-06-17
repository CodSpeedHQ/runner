use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use simplelog::SharedLogger;
use std::{env, fs};

use crate::prelude::*;
use crate::run::{
    ci_provider::{
        interfaces::{GhData, ProviderMetadata, RunEvent, Sender},
        provider::{CIProvider, CIProviderDetector},
    },
    config::Config,
    helpers::{find_repository_root, get_env_variable},
};

use super::logger::GithubActionLogger;

#[derive(Debug)]
pub struct GitHubActionsProvider {
    pub owner: String,
    pub repository: String,
    pub ref_: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub gh_data: GhData,
    pub event: RunEvent,
    pub repository_root_path: String,
}

impl GitHubActionsProvider {
    fn get_owner_and_repository() -> Result<(String, String)> {
        let owner_and_repository = get_env_variable("GITHUB_REPOSITORY")?;
        let mut owner_and_repository = owner_and_repository.split('/');
        let owner = owner_and_repository.next().unwrap();
        let repository = owner_and_repository.next().unwrap();
        Ok((owner.into(), repository.into()))
    }
}

lazy_static! {
    static ref PR_REF_REGEX: Regex = Regex::new(r"^refs/pull/(?P<pr_number>\d+)/merge$").unwrap();
}

impl TryFrom<&Config> for GitHubActionsProvider {
    type Error = Error;
    fn try_from(_config: &Config) -> Result<Self> {
        let (owner, repository) = Self::get_owner_and_repository()?;
        let ref_ = get_env_variable("GITHUB_REF")?;
        let is_pr = PR_REF_REGEX.is_match(&ref_);
        let head_ref = if is_pr {
            let github_event_path = get_env_variable("GITHUB_EVENT_PATH")?;
            let github_event = fs::read_to_string(github_event_path)?;
            let github_event: Value = serde_json::from_str(&github_event)
                .expect("GITHUB_EVENT_PATH file could not be read");
            let pull_request = github_event["pull_request"].as_object().unwrap();

            let head_repo = pull_request["head"]["repo"].as_object().unwrap();
            let base_repo = pull_request["base"]["repo"].as_object().unwrap();

            let is_head_repo_fork = head_repo["id"] != base_repo["id"];

            let head_ref = if is_head_repo_fork {
                format!(
                    "{}:{}",
                    head_repo["owner"]["login"].as_str().unwrap(),
                    pull_request["head"]["ref"].as_str().unwrap()
                )
            } else {
                pull_request["head"]["ref"].as_str().unwrap().to_owned()
            };
            Some(head_ref)
        } else {
            None
        };

        let github_event_name = get_env_variable("GITHUB_EVENT_NAME")?;
        let event = serde_json::from_str(&format!("\"{}\"", github_event_name)).context(
            format!("Event {} is not supported by CodSpeed", github_event_name),
        )?;
        let repository_root_path = match find_repository_root(&std::env::current_dir()?) {
            Some(mut path) => {
                // Add a trailing slash to the path
                path.push("");
                path.to_string_lossy().to_string()
            }
            None => format!("/home/runner/work/{}/{}/", repository, repository),
        };

        Ok(Self {
            owner,
            repository: repository.clone(),
            ref_,
            head_ref,
            event,
            gh_data: GhData {
                job: get_env_variable("GITHUB_JOB")?,
                run_id: get_env_variable("GITHUB_RUN_ID")?
                    .parse()
                    .context("Failed to parse GITHUB_RUN_ID into an integer")?,
                sender: Some(Sender {
                    login: get_env_variable("GITHUB_ACTOR")?,
                    id: get_env_variable("GITHUB_ACTOR_ID")?
                        .parse()
                        .context("Failed to parse GITHUB_ACTOR_ID into an integer")?,
                }),
            },
            base_ref: get_env_variable("GITHUB_BASE_REF").ok(),
            repository_root_path,
        })
    }
}

impl CIProviderDetector for GitHubActionsProvider {
    fn detect() -> bool {
        // check if the GITHUB_ACTIONS environment variable is set and the value is truthy
        env::var("GITHUB_ACTIONS") == Ok("true".into())
    }
}

impl CIProvider for GitHubActionsProvider {
    fn get_logger(&self) -> Box<dyn SharedLogger> {
        Box::new(GithubActionLogger)
    }

    fn get_provider_name(&self) -> &'static str {
        "GitHub Actions"
    }

    fn get_provider_slug(&self) -> &'static str {
        "github-actions"
    }

    fn get_provider_metadata(&self) -> Result<ProviderMetadata> {
        Ok(ProviderMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gh_data: Some(self.gh_data.clone()),
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use temp_env::{with_var, with_vars};

    use crate::VERSION;

    use super::*;

    #[test]
    fn test_detect() {
        with_var("GITHUB_ACTIONS", Some("true"), || {
            assert!(GitHubActionsProvider::detect());
        });
    }

    #[test]
    fn test_get_owner_and_repository() {
        with_var("GITHUB_REPOSITORY", Some("owner/repository"), || {
            let (owner, repository) = GitHubActionsProvider::get_owner_and_repository().unwrap();
            assert_eq!(owner, "owner");
            assert_eq!(repository, "repository");
        });
    }

    #[test]
    fn test_try_from_push_main() {
        with_vars(
            [
                ("GITHUB_ACTOR_ID", Some("1234567890")),
                ("GITHUB_ACTOR", Some("actor")),
                ("GITHUB_BASE_REF", Some("main")),
                ("GITHUB_EVENT_NAME", Some("push")),
                ("GITHUB_JOB", Some("job")),
                ("GITHUB_REF", Some("refs/heads/main")),
                ("GITHUB_REPOSITORY", Some("owner/repository")),
                ("GITHUB_RUN_ID", Some("1234567890")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                assert_eq!(github_actions_provider.owner, "owner");
                assert_eq!(github_actions_provider.repository, "repository");
                assert_eq!(github_actions_provider.ref_, "refs/heads/main");
                assert_eq!(github_actions_provider.base_ref, Some("main".into()));
                assert_eq!(github_actions_provider.head_ref, None);
                assert_eq!(github_actions_provider.event, RunEvent::Push);
                assert_eq!(github_actions_provider.gh_data.job, "job");
                assert_eq!(github_actions_provider.gh_data.run_id, 1234567890);
                assert_eq!(
                    github_actions_provider
                        .gh_data
                        .sender
                        .as_ref()
                        .unwrap()
                        .login,
                    "actor"
                );
                assert_eq!(
                    github_actions_provider.gh_data.sender.as_ref().unwrap().id,
                    1234567890
                );
            },
        )
    }

    #[test]
    fn test_pull_request_provider_metadata() {
        with_vars(
            [
                ("GITHUB_ACTIONS", Some("true")),
                ("GITHUB_ACTOR_ID", Some("19605940")),
                ("GITHUB_ACTOR", Some("adriencaccia")),
                ("GITHUB_BASE_REF", Some("main")),
                ("GITHUB_EVENT_NAME", Some("pull_request")),
                (
                    "GITHUB_EVENT_PATH",
                    Some(
                        format!(
                            "{}/src/run/ci_provider/github_actions/samples/pr-event.json",
                            env!("CARGO_MANIFEST_DIR")
                        )
                        .as_str(),
                    ),
                ),
                ("GITHUB_HEAD_REF", Some("feat/codspeed-runner")),
                ("GITHUB_JOB", Some("log-env")),
                ("GITHUB_REF", Some("refs/pull/22/merge")),
                ("GITHUB_REPOSITORY", Some("my-org/adrien-python-test")),
                ("GITHUB_RUN_ID", Some("6957110437")),
                ("VERSION", Some("0.1.0")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let provider_metadata = github_actions_provider.get_provider_metadata().unwrap();

                assert_json_snapshot!(provider_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }

    #[test]
    fn test_fork_pull_request_provider_metadata() {
        with_vars(
            [
                ("GITHUB_ACTIONS", Some("true")),
                ("GITHUB_ACTOR_ID", Some("19605940")),
                ("GITHUB_ACTOR", Some("adriencaccia")),
                ("GITHUB_BASE_REF", Some("main")),
                ("GITHUB_EVENT_NAME", Some("pull_request")),
                (
                    "GITHUB_EVENT_PATH",
                    Some(
                        format!(
                            "{}/src/run/ci_provider/github_actions/samples/fork-pr-event.json",
                            env!("CARGO_MANIFEST_DIR")
                        )
                        .as_str(),
                    ),
                ),
                ("GITHUB_HEAD_REF", Some("feat/codspeed-runner")),
                ("GITHUB_JOB", Some("log-env")),
                ("GITHUB_REF", Some("refs/pull/22/merge")),
                ("GITHUB_REPOSITORY", Some("my-org/adrien-python-test")),
                ("GITHUB_RUN_ID", Some("6957110437")),
                ("VERSION", Some("0.1.0")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let provider_metadata = github_actions_provider.get_provider_metadata().unwrap();

                assert_eq!(provider_metadata.owner, "my-org");
                assert_eq!(provider_metadata.repository, "adrien-python-test");
                assert_eq!(provider_metadata.base_ref, Some("main".into()));
                assert_eq!(
                    provider_metadata.head_ref,
                    Some("fork-owner:feat/codspeed-runner".into())
                );
                assert_json_snapshot!(provider_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }
}
