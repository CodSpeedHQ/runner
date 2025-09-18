use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use simplelog::SharedLogger;
use std::collections::BTreeMap;
use std::{env, fs};

use crate::prelude::*;
use crate::run::run_environment::{RunEnvironment, RunPart};
use crate::run::{
    config::Config,
    helpers::{find_repository_root, get_env_variable},
    run_environment::{
        interfaces::{GhData, RepositoryProvider, RunEnvironmentMetadata, RunEvent, Sender},
        provider::{RunEnvironmentDetector, RunEnvironmentProvider},
    },
};

use super::logger::GithubActionLogger;

#[derive(Debug)]
pub struct GitHubActionsProvider {
    pub owner: String,
    pub repository: String,
    pub ref_: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub sender: Option<Sender>,
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
    fn try_from(config: &Config) -> Result<Self> {
        if config.repository_override.is_some() {
            bail!("Specifying owner and repository from CLI is not supported for Github Actions");
        }
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
        let event = serde_json::from_str(&format!("\"{github_event_name}\"")).context(format!(
            "Event {github_event_name} is not supported by CodSpeed"
        ))?;
        let repository_root_path = match find_repository_root(&std::env::current_dir()?) {
            Some(mut path) => {
                // Add a trailing slash to the path
                path.push("");
                path.to_string_lossy().to_string()
            }
            None => format!("/home/runner/work/{repository}/{repository}/"),
        };

        Ok(Self {
            owner,
            repository: repository.clone(),
            ref_,
            head_ref,
            event,
            gh_data: GhData {
                job: get_env_variable("GITHUB_JOB")?,
                run_id: get_env_variable("GITHUB_RUN_ID")?,
            },
            sender: Some(Sender {
                login: get_env_variable("GITHUB_ACTOR")?,
                id: get_env_variable("GITHUB_ACTOR_ID")?,
            }),
            base_ref: get_env_variable("GITHUB_BASE_REF").ok(),
            repository_root_path,
        })
    }
}

impl RunEnvironmentDetector for GitHubActionsProvider {
    fn detect() -> bool {
        // check if the GITHUB_ACTIONS environment variable is set and the value is truthy
        env::var("GITHUB_ACTIONS") == Ok("true".into())
    }
}

impl RunEnvironmentProvider for GitHubActionsProvider {
    fn get_repository_provider(&self) -> RepositoryProvider {
        RepositoryProvider::GitHub
    }

    fn get_logger(&self) -> Box<dyn SharedLogger> {
        Box::new(GithubActionLogger::new())
    }

    fn get_run_environment(&self) -> RunEnvironment {
        RunEnvironment::GithubActions
    }

    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata> {
        Ok(RunEnvironmentMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gh_data: Some(self.gh_data.clone()),
            gl_data: None,
            sender: self.sender.clone(),
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
        })
    }

    /// For Github, the run environment run part is the most complicated
    /// since we support matrix jobs.
    ///
    /// Computing the `run_part_id`:
    /// - not in a matrix:
    ///   - simply take the job name
    /// - in a matrix:
    ///   - take the job name
    ///   - concatenate it with key-values from `matrix` and `strategy`
    ///
    /// `GH_MATRIX` and `GH_STRATEGY` are environment variables computed by
    /// https://github.com/CodSpeedHQ/action:
    /// - `GH_MATRIX`: ${{ toJson(matrix) }}
    /// - `GH_STRATEGY`: ${{ toJson(strategy) }}
    ///
    /// A note on parsing:
    ///
    /// The issue is these variables from Github Actions are multiline.
    /// As we need to use them compute an identifier, we need them as a single line.
    /// Plus we are interested in the content of these objects,
    /// so it makes sense to parse and re-serialize them.
    fn get_run_provider_run_part(&self) -> Option<RunPart> {
        let job_name = self.gh_data.job.clone();

        let mut metadata = BTreeMap::new();

        let gh_matrix = get_env_variable("GH_MATRIX")
            .ok()
            .and_then(|v| serde_json::from_str::<Value>(&v).ok());

        let gh_strategy = get_env_variable("GH_STRATEGY")
            .ok()
            .and_then(|v| serde_json::from_str::<Value>(&v).ok());

        let run_part_id = if let (Some(Value::Object(matrix)), Some(Value::Object(mut strategy))) =
            (gh_matrix, gh_strategy)
        {
            // remove useless values from the strategy
            strategy.remove("fail-fast");
            strategy.remove("max-parallel");

            // The re-serialization is on purpose here. We want to serialize it as a single line.
            let matrix_str = serde_json::to_string(&matrix).expect("Unable to re-serialize matrix");
            let strategy_str =
                serde_json::to_string(&strategy).expect("Unable to re-serialize strategy");

            metadata.extend(matrix);
            metadata.extend(strategy);

            format!("{job_name}-{matrix_str}-{strategy_str}")
        } else {
            job_name
        };

        Some(RunPart {
            run_id: self.gh_data.run_id.clone(),
            run_part_id,
            job_name: self.gh_data.job.clone(),
            metadata,
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
                assert_eq!(github_actions_provider.gh_data.run_id, "1234567890");
                assert_eq!(
                    github_actions_provider.sender.as_ref().unwrap().login,
                    "actor"
                );
                assert_eq!(
                    github_actions_provider.sender.as_ref().unwrap().id,
                    "1234567890"
                );
            },
        )
    }

    #[test]
    fn test_pull_request_run_environment_metadata() {
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
                            "{}/src/run/run_environment/github_actions/samples/pr-event.json",
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
                let run_environment_metadata = github_actions_provider
                    .get_run_environment_metadata()
                    .unwrap();
                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
                assert_json_snapshot!(run_part);
            },
        );
    }

    #[test]
    fn test_fork_pull_request_run_environment_metadata() {
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
                            "{}/src/run/run_environment/github_actions/samples/fork-pr-event.json",
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
                ("GH_MATRIX", Some("null")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let run_environment_metadata = github_actions_provider
                    .get_run_environment_metadata()
                    .unwrap();
                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_eq!(run_environment_metadata.owner, "my-org");
                assert_eq!(run_environment_metadata.repository, "adrien-python-test");
                assert_eq!(run_environment_metadata.base_ref, Some("main".into()));
                assert_eq!(
                    run_environment_metadata.head_ref,
                    Some("fork-owner:feat/codspeed-runner".into())
                );

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
                assert_json_snapshot!(run_part);
            },
        );
    }

    #[test]
    fn test_matrix_job_run_environment_metadata() {
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
                            "{}/src/run/run_environment/github_actions/samples/pr-event.json",
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
                (
                    "GH_MATRIX",
                    Some(
                        r#"{
    "runner-version":"3.2.1",
    "numeric-value":123456789
}"#,
                    ),
                ),
                (
                    "GH_STRATEGY",
                    Some(
                        r#"{
    "fail-fast":true,
    "job-index":1,
    "job-total":2,
    "max-parallel":2
}"#,
                    ),
                ),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let github_actions_provider = GitHubActionsProvider::try_from(&config).unwrap();
                let run_environment_metadata = github_actions_provider
                    .get_run_environment_metadata()
                    .unwrap();
                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
                assert_json_snapshot!(run_part);
            },
        );
    }

    #[test]
    fn test_get_run_part_no_matrix() {
        with_vars([("GITHUB_ACTIONS", Some("true"))], || {
            let github_actions_provider = GitHubActionsProvider {
                owner: "owner".into(),
                repository: "repository".into(),
                ref_: "refs/head/my-branch".into(),
                head_ref: Some("my-branch".into()),
                base_ref: None,
                sender: None,
                gh_data: GhData {
                    job: "my_job".into(),
                    run_id: "123789".into(),
                },
                event: RunEvent::Push,
                repository_root_path: "/home/work/my-repo".into(),
            };

            let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

            assert_eq!(run_part.run_id, "123789");
            assert_eq!(run_part.job_name, "my_job");
            assert_eq!(run_part.run_part_id, "my_job");
            assert_json_snapshot!(run_part.metadata, @"{}");
        })
    }

    #[test]
    fn test_get_run_part_null_matrix() {
        with_vars(
            [
                ("GH_MATRIX", Some("null")),
                (
                    "GH_STRATEGY",
                    Some(
                        r#"{
    "fail-fast":true,
    "job-index":0,
    "job-total":1,
    "max-parallel":1
}"#,
                    ),
                ),
            ],
            || {
                let github_actions_provider = GitHubActionsProvider {
                    owner: "owner".into(),
                    repository: "repository".into(),
                    ref_: "refs/head/my-branch".into(),
                    head_ref: Some("my-branch".into()),
                    base_ref: None,
                    sender: None,
                    gh_data: GhData {
                        job: "my_job".into(),
                        run_id: "123789".into(),
                    },
                    event: RunEvent::Push,
                    repository_root_path: "/home/work/my-repo".into(),
                };

                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_eq!(run_part.run_id, "123789");
                assert_eq!(run_part.job_name, "my_job");
                assert_eq!(run_part.run_part_id, "my_job");
                assert_json_snapshot!(run_part.metadata, @"{}");
            },
        )
    }

    #[test]
    fn test_get_matrix_run_part() {
        with_vars(
            [
                (
                    "GH_MATRIX",
                    Some(
                        r#"{
    "runner-version":"3.2.1",
    "numeric-value":123456789
}"#,
                    ),
                ),
                (
                    "GH_STRATEGY",
                    Some(
                        r#"{
    "fail-fast":true,
    "job-index":1,
    "job-total":2,
    "max-parallel":2
}"#,
                    ),
                ),
            ],
            || {
                let github_actions_provider = GitHubActionsProvider {
                    owner: "owner".into(),
                    repository: "repository".into(),
                    ref_: "refs/head/my-branch".into(),
                    head_ref: Some("my-branch".into()),
                    base_ref: None,
                    sender: None,
                    gh_data: GhData {
                        job: "my_job".into(),
                        run_id: "123789".into(),
                    },
                    event: RunEvent::Push,
                    repository_root_path: "/home/work/my-repo".into(),
                };

                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_eq!(run_part.run_id, "123789");
                assert_eq!(run_part.job_name, "my_job");
                assert_eq!(
                    run_part.run_part_id,
                    "my_job-{\"runner-version\":\"3.2.1\",\"numeric-value\":123456789}-{\"job-total\":2,\"job-index\":1}"
                );
                assert_json_snapshot!(run_part.metadata, @r#"
                {
                  "job-index": 1,
                  "job-total": 2,
                  "numeric-value": 123456789,
                  "runner-version": "3.2.1"
                }
                "#);
            },
        )
    }

    #[test]
    fn test_get_inline_matrix_run_part() {
        with_vars(
            [
                (
                    "GH_MATRIX",
                    Some("{\"runner-version\":\"3.2.1\",\"numeric-value\":123456789}"),
                ),
                (
                    "GH_STRATEGY",
                    Some("{\"fail-fast\":true,\"job-index\":1,\"job-total\":2,\"max-parallel\":2}"),
                ),
            ],
            || {
                let github_actions_provider = GitHubActionsProvider {
                    owner: "owner".into(),
                    repository: "repository".into(),
                    ref_: "refs/head/my-branch".into(),
                    head_ref: Some("my-branch".into()),
                    base_ref: None,
                    sender: None,
                    gh_data: GhData {
                        job: "my_job".into(),
                        run_id: "123789".into(),
                    },
                    event: RunEvent::Push,
                    repository_root_path: "/home/work/my-repo".into(),
                };

                let run_part = github_actions_provider.get_run_provider_run_part().unwrap();

                assert_eq!(run_part.run_id, "123789");
                assert_eq!(run_part.job_name, "my_job");
                assert_eq!(
                    run_part.run_part_id,
                    "my_job-{\"runner-version\":\"3.2.1\",\"numeric-value\":123456789}-{\"job-total\":2,\"job-index\":1}"
                );
                assert_json_snapshot!(run_part.metadata, @r#"
                {
                  "job-index": 1,
                  "job-total": 2,
                  "numeric-value": 123456789,
                  "runner-version": "3.2.1"
                }
                "#);
            },
        )
    }
}
