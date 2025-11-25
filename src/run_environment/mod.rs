pub mod interfaces;
pub mod logger;
mod provider;

use buildkite::BuildkiteProvider;
use github_actions::GitHubActionsProvider;
use gitlab_ci::GitLabCIProvider;
use local::LocalProvider;
use provider::RunEnvironmentDetector;

use crate::executor::Config;
use crate::prelude::*;

pub use self::interfaces::*;
pub use self::provider::RunEnvironmentProvider;

// RunEnvironment Provider implementations
mod buildkite;
mod github_actions;
mod gitlab_ci;
mod local;

pub fn get_provider(config: &Config) -> Result<Box<dyn RunEnvironmentProvider>> {
    if BuildkiteProvider::detect() {
        let provider = BuildkiteProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if GitLabCIProvider::detect() {
        let provider = GitLabCIProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if LocalProvider::detect() {
        let provider = LocalProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    // By design, this should not happen as the `LocalProvider` is a fallback
    bail!("No RunEnvironment provider detected")
}
