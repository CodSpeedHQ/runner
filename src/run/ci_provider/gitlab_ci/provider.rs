use simplelog::SharedLogger;
use std::env;

use crate::prelude::*;
use crate::run::ci_provider::interfaces::ProviderMetadata;
use crate::run::ci_provider::provider::CIProviderDetector;
use crate::run::ci_provider::CIProvider;
use crate::run::config::Config;

use super::logger::GitLabCILogger;

#[derive(Debug)]
pub struct GitLabCIProvider {}

impl TryFrom<&Config> for GitLabCIProvider {
    type Error = Error;
    fn try_from(_config: &Config) -> Result<Self> {
        Ok(Self {})
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
        unimplemented!()
    }
}
