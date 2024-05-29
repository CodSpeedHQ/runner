pub mod interfaces;
pub mod logger;
mod provider;

use buildkite::BuildkiteProvider;
use github_actions::GitHubActionsProvider;
use local::LocalProvider;
use provider::CIProviderDetector;

use crate::prelude::*;
use crate::run::config::Config;

pub use self::provider::CIProvider;

// Provider implementations
mod buildkite;
mod github_actions;
mod local;

pub fn get_provider(config: &Config) -> Result<Box<dyn CIProvider>> {
    if BuildkiteProvider::detect() {
        let provider = BuildkiteProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    if LocalProvider::detect() {
        let provider = LocalProvider::try_from(config)?;
        return Ok(Box::new(provider));
    }

    bail!("No CI provider detected")
}
