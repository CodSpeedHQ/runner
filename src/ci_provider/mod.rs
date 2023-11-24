pub mod logger;
mod provider;

use crate::ci_provider::github_actions::GitHubActionsProvider;
use crate::config::Config;
use crate::prelude::*;

pub use self::provider::CIProvider;

// Provider implementations
mod github_actions;

pub fn get_provider(config: &Config) -> Result<impl CIProvider> {
    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(provider);
    }

    bail!("No CI provider detected")
}
