use simplelog::SharedLogger;
use std::env;

use crate::prelude::*;
use crate::run::config::Config;
use crate::run::helpers::get_env_variable;
use crate::run::run_environment::interfaces::{
    GlData, RepositoryProvider, RunEnvironment, RunEnvironmentMetadata, RunEvent, Sender,
};
use crate::run::run_environment::provider::RunEnvironmentDetector;
use crate::run::run_environment::RunEnvironmentProvider;

use super::logger::GitLabCILogger;

#[derive(Debug)]
pub struct GitLabCIProvider {
    owner: String,
    repository: String,
    ref_: String,
    head_ref: Option<String>,
    base_ref: Option<String>,
    gl_data: GlData,
    sender: Sender,
    event: RunEvent,
    repository_root_path: String,
}

impl TryFrom<&Config> for GitLabCIProvider {
    type Error = Error;
    fn try_from(_config: &Config) -> Result<Self> {
        let owner = get_env_variable("CI_PROJECT_NAMESPACE")?;
        let repository = get_env_variable("CI_PROJECT_NAME")?;

        let ci_pipeline_source = get_env_variable("CI_PIPELINE_SOURCE")?;

        let branch_name = get_env_variable("CI_COMMIT_REF_NAME")?;

        // compute the branch or tag ref which mimics GitHub behavior
        // CI_COMMIT_TAG is only present in pipelines for tags.
        // See https://docs.gitlab.com/ee/ci/variables/predefined_variables.html
        let branch_or_tag_ref = get_env_variable("CI_COMMIT_TAG")
            .map(|tag_name| format!("refs/tags/{tag_name}"))
            .unwrap_or(format!("refs/heads/{branch_name}"));

        // https://docs.gitlab.com/ee/ci/jobs/job_rules.html#ci_pipeline_source-predefined-variable
        let (event, ref_, base_ref, head_ref) = match ci_pipeline_source.as_str() {
            // For pipelines created when a merge request is created or updated. Required to enable merge request pipelines, merged results pipelines, and merge trains.
            // https://docs.gitlab.com/ee/ci/variables/predefined_variables.html#predefined-variables-for-merge-request-pipelines
            "merge_request_event" => {
                let merge_request_id = get_env_variable("CI_MERGE_REQUEST_IID")?;
                let target_branch_name = get_env_variable("CI_MERGE_REQUEST_TARGET_BRANCH_NAME")?;
                let source_branch_name = get_env_variable("CI_MERGE_REQUEST_SOURCE_BRANCH_NAME")?;

                // check if the merge request is from a fork
                let ci_project_path = get_env_variable("CI_PROJECT_PATH")?;
                let ci_merge_request_source_project_path =
                    get_env_variable("CI_MERGE_REQUEST_SOURCE_PROJECT_PATH")?;

                if ci_project_path != ci_merge_request_source_project_path {
                    let fork_owner = ci_merge_request_source_project_path
                        .split('/')
                        .next()
                        .expect("Malformed Source Project Path");

                    (
                        RunEvent::PullRequest,
                        format!("refs/pull/{merge_request_id}/merge"),
                        Some(target_branch_name),
                        Some(format!("{fork_owner}:{source_branch_name}")),
                    )
                } else {
                    (
                        RunEvent::PullRequest,
                        format!("refs/pull/{merge_request_id}/merge"),
                        Some(target_branch_name),
                        Some(source_branch_name),
                    )
                }
            }

            // For pipelines triggered by a Git push event, including for branches and tags.
            "push" => (RunEvent::Push, branch_or_tag_ref, Some(branch_name), None),

            // For scheduled pipelines.
            "schedule" => (
                RunEvent::Schedule,
                branch_or_tag_ref,
                Some(branch_name),
                None,
            ),

            // For pipelines created with the api, using a trigger token or via the GitLab UI.
            "trigger" | "web" | "api" => (
                RunEvent::WorkflowDispatch,
                branch_or_tag_ref,
                Some(branch_name),
                None,
            ),

            _ => bail!("Event {} is not supported by CodSpeed", ci_pipeline_source),
        };

        let run_id = get_env_variable("CI_JOB_ID")?;
        let job = get_env_variable("CI_JOB_NAME")?;

        let gitlab_user_id = get_env_variable("GITLAB_USER_ID")?;
        let gitlab_user_login = get_env_variable("GITLAB_USER_LOGIN")?;

        let gl_data = GlData { run_id, job };
        let sender = Sender {
            id: gitlab_user_id,
            login: gitlab_user_login,
        };

        let repository_root_path = get_env_variable("CI_PROJECT_DIR")?;

        Ok(Self {
            owner,
            repository,
            ref_,
            head_ref,
            base_ref,
            gl_data,
            sender,
            event,
            repository_root_path,
        })
    }
}

impl RunEnvironmentDetector for GitLabCIProvider {
    fn detect() -> bool {
        // check if the GITLAB_CI environment variable is set and the value is truthy
        env::var("GITLAB_CI") == Ok("true".into())
    }
}

impl RunEnvironmentProvider for GitLabCIProvider {
    fn get_logger(&self) -> Box<dyn SharedLogger> {
        Box::new(GitLabCILogger::new())
    }

    fn get_repository_provider(&self) -> RepositoryProvider {
        RepositoryProvider::GitLab
    }

    fn get_run_environment_name(&self) -> &'static str {
        "GitLab CI"
    }

    fn get_run_environment(&self) -> RunEnvironment {
        RunEnvironment::GitlabCi
    }

    fn get_run_environment_metadata(&self) -> Result<RunEnvironmentMetadata> {
        Ok(RunEnvironmentMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gh_data: None,
            gl_data: Some(self.gl_data.clone()),
            sender: Some(self.sender.clone()),
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
        with_var("GITLAB_CI", Some("true"), || {
            assert!(GitLabCIProvider::detect());
        });
    }

    #[test]
    fn test_push_main_run_environment_metadata() {
        with_vars(
            [
                ("GITLAB_CI", Some("true")),
                ("CI_PROJECT_DIR", Some("/builds/owner/repository")),
                ("GITLAB_USER_ID", Some("1234567890")),
                ("GITLAB_USER_LOGIN", Some("actor")),
                ("CI_PROJECT_NAME", Some("repository")),
                ("CI_PROJECT_NAMESPACE", Some("owner")),
                ("CI_JOB_NAME", Some("job")),
                ("CI_JOB_ID", Some("1234567890")),
                ("CI_PIPELINE_SOURCE", Some("push")),
                ("CI_COMMIT_REF_NAME", Some("main")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let gitlab_ci_provider = GitLabCIProvider::try_from(&config).unwrap();
                let run_environment_metadata =
                    gitlab_ci_provider.get_run_environment_metadata().unwrap();

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        )
    }

    #[test]
    fn test_merge_request_run_environment_metadata() {
        with_vars(
            [
                ("GITLAB_CI", Some("true")),
                ("CI_PROJECT_DIR", Some("/builds/owner/repository")),
                ("GITLAB_USER_ID", Some("19605940")),
                ("GITLAB_USER_LOGIN", Some("actor")),
                ("CI_PROJECT_NAME", Some("repository")),
                ("CI_PROJECT_NAMESPACE", Some("owner")),
                ("CI_JOB_NAME", Some("build-job")),
                ("CI_JOB_ID", Some("6957110437")),
                ("CI_PIPELINE_SOURCE", Some("merge_request_event")),
                ("CI_COMMIT_REF_NAME", Some("main")),
                ("CI_MERGE_REQUEST_IID", Some("5")),
                ("CI_MERGE_REQUEST_TARGET_BRANCH_NAME", Some("main")),
                (
                    "CI_MERGE_REQUEST_SOURCE_BRANCH_NAME",
                    Some("feat/awesome-feature"),
                ),
                ("CI_PROJECT_PATH", Some("owner/repository")),
                (
                    "CI_MERGE_REQUEST_SOURCE_PROJECT_PATH",
                    Some("owner/repository"),
                ),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let gitlab_ci_provider = GitLabCIProvider::try_from(&config).unwrap();
                let run_environment_metadata =
                    gitlab_ci_provider.get_run_environment_metadata().unwrap();

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }

    #[test]
    fn test_fork_merge_request_run_environment_metadata() {
        with_vars(
            [
                ("GITLAB_CI", Some("true")),
                ("CI_PROJECT_DIR", Some("/builds/owner/repository")),
                ("GITLAB_USER_ID", Some("19605940")),
                ("GITLAB_USER_LOGIN", Some("actor")),
                ("CI_PROJECT_NAME", Some("repository")),
                ("CI_PROJECT_NAMESPACE", Some("owner")),
                ("CI_JOB_NAME", Some("build-job")),
                ("CI_JOB_ID", Some("6957110437")),
                ("CI_PIPELINE_SOURCE", Some("merge_request_event")),
                ("CI_COMMIT_REF_NAME", Some("main")),
                ("CI_MERGE_REQUEST_IID", Some("5")),
                ("CI_MERGE_REQUEST_TARGET_BRANCH_NAME", Some("main")),
                (
                    "CI_MERGE_REQUEST_SOURCE_BRANCH_NAME",
                    Some("feat/awesome-feature"),
                ),
                ("CI_PROJECT_PATH", Some("owner/repository")),
                (
                    "CI_MERGE_REQUEST_SOURCE_PROJECT_PATH",
                    Some("fork-owner/fork-repository"),
                ),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let gitlab_ci_provider = GitLabCIProvider::try_from(&config).unwrap();
                let run_environment_metadata =
                    gitlab_ci_provider.get_run_environment_metadata().unwrap();

                assert_json_snapshot!(run_environment_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                });
            },
        );
    }
}
