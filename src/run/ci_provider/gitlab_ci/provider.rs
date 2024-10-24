use simplelog::SharedLogger;
use std::env;

use crate::prelude::*;
use crate::run::ci_provider::interfaces::{GlData, GlSender, ProviderMetadata, RunEvent};
use crate::run::ci_provider::provider::CIProviderDetector;
use crate::run::ci_provider::CIProvider;
use crate::run::config::Config;
use crate::run::helpers::get_env_variable;

use super::logger::GitLabCILogger;

#[derive(Debug)]
pub struct GitLabCIProvider {
    owner: String,
    repository: String,
    ref_: String,
    head_ref: Option<String>,
    base_ref: Option<String>,
    gl_data: GlData,
    event: RunEvent,
    repository_root_path: String,
}

impl TryFrom<&Config> for GitLabCIProvider {
    type Error = Error;
    fn try_from(_config: &Config) -> Result<Self> {
        let owner = get_env_variable("CI_PROJECT_NAMESPACE")?;
        let repository = get_env_variable("CI_PROJECT_NAME")?;
        let ref_ = get_env_variable("CI_COMMIT_REF_NAME")?;

        let ci_pipeline_source = get_env_variable("CI_PIPELINE_SOURCE")?;

        // https://docs.gitlab.com/ee/ci/jobs/job_rules.html#ci_pipeline_source-predefined-variable
        let event = match ci_pipeline_source.as_str() {
            "external_pull_request_event" | "merge_request_event" => RunEvent::PullRequest,
            "push" => RunEvent::Push,
            "schedule" => RunEvent::Schedule,
            "trigger" | "web" => RunEvent::WorkflowDispatch,

            _ => bail!("Event {} is not supported by CodSpeed", ci_pipeline_source),
        };

        let run_id = get_env_variable("CI_JOB_ID")?;
        let job = get_env_variable("CI_JOB_NAME")?;

        let gitlab_user_id = get_env_variable("GITLAB_USER_ID")?;
        let gitlab_user_login = get_env_variable("GITLAB_USER_LOGIN")?;

        let gl_data = GlData {
            run_id,
            job,
            sender: Some(GlSender {
                id: gitlab_user_id,
                login: gitlab_user_login,
            }),
        };

        let repository_root_path = get_env_variable("PWD")?;

        Ok(Self {
            owner,
            repository,
            ref_,
            head_ref: None,
            base_ref: None,
            gl_data,
            event,
            repository_root_path,
        })
    }
}

impl CIProviderDetector for GitLabCIProvider {
    fn detect() -> bool {
        // check if the GITLAB_CI environment variable is set and the value is truthy
        env::var("GITLAB_CI") == Ok("true".into())
    }
}

impl CIProvider for GitLabCIProvider {
    fn get_logger(&self) -> Box<dyn SharedLogger> {
        Box::new(GitLabCILogger::new())
    }

    fn get_provider_name(&self) -> &'static str {
        "GitLab CI"
    }

    fn get_provider_slug(&self) -> &'static str {
        "gitlab-ci"
    }

    fn get_provider_metadata(&self) -> Result<ProviderMetadata> {
        Ok(ProviderMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            gl_data: Some(self.gl_data.clone()),
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
            gh_data: None,
        })
    }
}
