use std::{env, fs};

use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;

use crate::{
    ci_provider::provider::{CIProvider, CIProviderDetector},
    config::Config,
    helpers::get_env_variable,
    prelude::*,
    uploader::{GhData, Runner, Sender, UploadMetadata},
    VERSION,
};

use super::logger::GithubActionLogger;

#[derive(Debug)]
pub struct GitHubActionsProvider {
    pub owner: String,
    pub repository: String,
    pub ref_: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub commit_hash: String,
    pub gh_data: GhData,
    pub event: String,
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
        let (head_ref, commit_hash) = if is_pr {
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
            (
                Some(head_ref),
                pull_request["head"]["sha"].as_str().unwrap().to_owned(),
            )
        } else {
            (None, get_env_variable("GITHUB_SHA")?)
        };

        Ok(Self {
            owner,
            repository,
            ref_,
            commit_hash,
            head_ref,
            event: get_env_variable("GITHUB_EVENT_NAME")?,
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
    fn setup_logger(&self) -> Result<()> {
        log::set_logger(&GithubActionLogger)?;
        log::set_max_level(log::LevelFilter::Trace);
        Ok(())
    }

    fn get_provider_name(&self) -> &'static str {
        "GitHub Actions"
    }

    fn get_provider_slug(&self) -> &'static str {
        "github-actions"
    }

    fn get_upload_metadata(&self, config: &Config, archive_hash: &str) -> Result<UploadMetadata> {
        let upload_metadata = UploadMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            commit_hash: self.commit_hash.clone(),
            event: self.event.clone(),
            gh_data: self.gh_data.clone(),
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),

            // TODO: refactor in a default implementation of the trait, as it will be the same for all providers
            runner: Runner {
                name: "codspeed-runner".into(),
                // TODO add back when integrating another provider
                // platform: self.get_provider_slug().into(),
                version: VERSION.to_string(),
            },
            tokenless: config.token.is_none(),
            version: Some(1),
            profile_md5: archive_hash.to_string(),
        };

        Ok(upload_metadata)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use temp_env::{with_var, with_vars};
    use url::Url;

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
                ("GITHUB_SHA", Some("1234567890abcdef")),
            ],
            || {
                let config = Config {
                    command: "upload".into(),
                    skip_setup: false,
                    skip_upload: false,
                    token: Some("token".into()),
                    upload_url: Url::parse("https://example.com").unwrap(),
                    working_directory: Some(".".into()),
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                assert_eq!(github_actions_provider.owner, "owner");
                assert_eq!(github_actions_provider.repository, "repository");
                assert_eq!(github_actions_provider.ref_, "refs/heads/main");
                assert_eq!(github_actions_provider.base_ref, Some("main".into()));
                assert_eq!(github_actions_provider.head_ref, None);
                assert_eq!(github_actions_provider.commit_hash, "1234567890abcdef");
                assert_eq!(github_actions_provider.event, "push");
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
    fn test_pull_request_upload_metadata() {
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
                            "{}/src/ci_provider/github_actions/samples/pr-event.json",
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
                (
                    "GITHUB_SHA",
                    Some("5bd77cb0da72bef094893ed45fb793ff16ecfbe3"),
                ),
                ("VERSION", Some("0.1.0")),
            ],
            || {
                let config = Config {
                    command: "upload".into(),
                    skip_setup: false,
                    skip_upload: false,
                    token: Some("token".into()),
                    upload_url: Url::parse("https://example.com").unwrap(),
                    working_directory: Some(".".into()),
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let upload_metadata = github_actions_provider
                    .get_upload_metadata(&config, "archive_hash")
                    .unwrap();

                assert_json_snapshot!(upload_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }

    #[test]
    fn test_fork_pull_request_upload_metadata() {
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
                            "{}/src/ci_provider/github_actions/samples/fork-pr-event.json",
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
                (
                    "GITHUB_SHA",
                    Some("5bd77cb0da72bef094893ed45fb793ff16ecfbe3"),
                ),
                ("VERSION", Some("0.1.0")),
            ],
            || {
                let config = Config {
                    command: "upload".into(),
                    skip_setup: false,
                    skip_upload: false,
                    token: Some("token".into()),
                    upload_url: Url::parse("https://example.com").unwrap(),
                    working_directory: Some(".".into()),
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let upload_metadata = github_actions_provider
                    .get_upload_metadata(&config, "archive_hash")
                    .unwrap();

                assert_eq!(upload_metadata.owner, "my-org");
                assert_eq!(upload_metadata.repository, "adrien-python-test");
                assert_eq!(upload_metadata.base_ref, Some("main".into()));
                assert_eq!(
                    upload_metadata.head_ref,
                    Some("fork-owner:feat/codspeed-runner".into())
                );
                assert_json_snapshot!(upload_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }
}
