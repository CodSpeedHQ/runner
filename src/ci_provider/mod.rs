pub mod interfaces;
pub mod logger;
mod provider;

use crate::ci_provider::buildkite::BuildkiteProvider;
use crate::ci_provider::github_actions::GitHubActionsProvider;
use crate::ci_provider::provider::CIProviderDetector;
use crate::config::Config;
use crate::prelude::*;

pub use self::provider::CIProvider;

// Provider implementations
mod buildkite;
mod github_actions;

pub fn get_provider(config: &Config) -> Result<Box<dyn CIProvider>> {
    if BuildkiteProvider::detect() {
        let provider = BuildkiteProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    bail!("No CI provider detected")
}
