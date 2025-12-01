use std::fmt::Display;

pub mod config;
mod helpers;
mod interfaces;
#[cfg(test)]
mod tests;
mod valgrind;
mod wall_time;

use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::instruments::mongo_tracer::MongoTracer;
use crate::prelude::*;
use crate::run::check_system::{self, SystemInfo};
use crate::run::logger::Logger;
use crate::run::{poll_results, show_banner, uploader};
use crate::run_environment::{self, RunEnvironment};
use crate::runner_mode::RunnerMode;
use async_trait::async_trait;
pub use config::Config;
pub use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutionContext, ExecutorName, RunData};
use std::path::Path;
use valgrind::executor::ValgrindExecutor;
use wall_time::executor::WallTimeExecutor;

impl Display for RunnerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[allow(deprecated)]
            RunnerMode::Instrumentation => write!(f, "instrumentation"),
            RunnerMode::Simulation => write!(f, "simulation"),
            RunnerMode::Walltime => write!(f, "walltime"),
        }
    }
}

pub const EXECUTOR_TARGET: &str = "executor";

pub fn get_executor_from_mode(
    mode: &RunnerMode,
    start_executor_with_instrumentation_enabled: bool,
) -> Box<dyn Executor> {
    match mode {
        #[allow(deprecated)]
        RunnerMode::Instrumentation | RunnerMode::Simulation => Box::new(ValgrindExecutor),
        RunnerMode::Walltime => Box::new(WallTimeExecutor::new(
            start_executor_with_instrumentation_enabled,
        )),
    }
}

pub fn get_all_executors() -> Vec<Box<dyn Executor>> {
    vec![
        Box::new(ValgrindExecutor),
        Box::new(WallTimeExecutor::new(false)),
    ]
}

pub fn get_run_data(config: &Config) -> Result<RunData> {
    let profile_folder = if let Some(profile_folder) = &config.profile_folder {
        profile_folder.clone()
    } else {
        create_profile_folder()?
    };
    Ok(RunData { profile_folder })
}

/// Initialize the execution environment (provider, logger, auth, system checks)
///
/// This phase:
/// - Detects the run environment provider
/// - Sets up logging
/// - Handles authentication (local token or OIDC check)
/// - Validates system compatibility
///
/// Returns an ExecutionContext with all necessary state for subsequent phases.
pub async fn initialize_execution_environment(
    mut config: Config,
    codspeed_config: &CodSpeedConfig,
) -> Result<ExecutionContext> {
    let mut provider = run_environment::get_provider(&config)?;
    let logger = Logger::new(&provider)?;

    #[allow(deprecated)]
    if config.mode == RunnerMode::Instrumentation {
        warn!(
            "The 'instrumentation' runner mode is deprecated and will be removed in a future version. \
                Please use 'simulation' instead."
        );
    }

    if provider.get_run_environment() != RunEnvironment::Local {
        show_banner();
    }
    debug!("config: {config:#?}");

    if provider.get_run_environment() == RunEnvironment::Local {
        if codspeed_config.auth.token.is_none() {
            bail!("You have to authenticate the CLI first. Run `codspeed auth login`.");
        }
        debug!("Using the token from the CodSpeed configuration file");
        config.set_token(codspeed_config.auth.token.clone());
    } else {
        provider.check_oidc_configuration(&config)?;
    }

    let system_info = SystemInfo::new()?;
    check_system::check_system(&system_info)?;

    let run_data = get_run_data(&config)?;

    Ok(ExecutionContext {
        config,
        provider,
        logger,
        system_info,
        run_data,
    })
}

/// Upload results and poll for completion
///
/// This phase:
/// - Sets OIDC token if needed (non-local environments)
/// - Uploads performance data
/// - Polls results (local environments only)
pub async fn upload_and_poll_results(
    executor: &dyn Executor,
    context: &mut ExecutionContext,
    api_client: &CodSpeedAPIClient,
    output_json: bool,
) -> Result<()> {
    // Set OIDC token just before upload (to avoid expiration)
    // Note: OIDC tokens can expire quickly, so we set it just before the upload
    if context.provider.get_run_environment() != RunEnvironment::Local {
        context.provider.set_oidc_token(&mut context.config).await?;
    }

    start_group!("Uploading performance data");
    let upload_result = uploader::upload(
        &context.config,
        &context.system_info,
        &context.provider,
        &context.run_data,
        executor.name(),
    )
    .await?;
    end_group!();

    if context.provider.get_run_environment() == RunEnvironment::Local {
        poll_results::poll_results(
            api_client,
            &context.provider,
            upload_result.run_id,
            output_json,
        )
        .await?;
        end_group!();
    }

    Ok(())
}

#[async_trait(?Send)]
pub trait Executor {
    fn name(&self) -> ExecutorName;

    fn start_with_instrumentation_enabled(&self) -> bool;

    async fn setup(
        &self,
        _system_info: &SystemInfo,
        _setup_cache_dir: Option<&Path>,
    ) -> Result<()> {
        Ok(())
    }

    /// Runs the executor
    async fn run(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        run_data: &RunData,
        // TODO: use Instruments instead of directly passing the mongodb tracer
        mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()>;

    async fn teardown(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()>;
}
