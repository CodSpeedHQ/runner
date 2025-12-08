use super::Config;
use crate::config::CodSpeedConfig;
use crate::prelude::*;
use crate::run::check_system::{self, SystemInfo};
use crate::run::logger::Logger;
use crate::run_environment::{self, RunEnvironment};
use std::path::PathBuf;

use super::create_profile_folder;

/// Runtime context for benchmark execution.
///
/// This struct contains all the necessary information and dependencies needed to execute
/// benchmarks, including the execution configuration, system information, environment provider,
/// and logging facilities. It is constructed from a [`Config`] and [`CodSpeedConfig`] and
/// serves as the primary context passed to executors during the benchmark run lifecycle.
pub struct ExecutionContext {
    pub config: Config,
    /// Directory path where profiling data and results are stored
    pub profile_folder: PathBuf,
    pub system_info: SystemInfo,
    /// The run environment provider (GitHub Actions, GitLab CI, local, etc.)
    pub provider: Box<dyn crate::run_environment::RunEnvironmentProvider>,
    pub logger: Logger,
}

impl ExecutionContext {
    pub fn is_local(&self) -> bool {
        self.provider.get_run_environment() == RunEnvironment::Local
    }
}

impl TryFrom<(Config, &CodSpeedConfig)> for ExecutionContext {
    type Error = anyhow::Error;

    fn try_from(
        (mut config, codspeed_config): (Config, &CodSpeedConfig),
    ) -> Result<Self, Self::Error> {
        let provider = run_environment::get_provider(&config)?;
        let system_info = SystemInfo::new()?;
        check_system::check_system(&system_info)?;
        let logger = Logger::new(provider.as_ref())?;

        let profile_folder = if let Some(profile_folder) = &config.profile_folder {
            profile_folder.clone()
        } else {
            create_profile_folder()?
        };

        if provider.get_run_environment() == RunEnvironment::Local {
            if codspeed_config.auth.token.is_none() {
                bail!("You have to authenticate the CLI first. Run `codspeed auth login`.");
            }
            debug!("Using the token from the CodSpeed configuration file");
            config.set_token(codspeed_config.auth.token.clone());
        }

        Ok(ExecutionContext {
            config,
            profile_folder,
            system_info,
            provider,
            logger,
        })
    }
}
