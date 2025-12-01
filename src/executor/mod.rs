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
use crate::instruments::mongo_tracer::{MongoTracer, install_mongodb_tracer};
use crate::prelude::*;
use crate::run::check_system::{self, SystemInfo};
use crate::run::logger::Logger;
use crate::run::{poll_results, show_banner, uploader};
use crate::run_environment::{self, RunEnvironment};
use crate::runner_mode::RunnerMode;
use async_trait::async_trait;
pub use config::Config;
pub use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutorName, RunData};
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

/// Execute benchmarks with the given configuration
/// This is the core execution logic shared between `run` and `exec` commands
pub async fn execute_benchmarks(
    mut config: Config,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
    setup_cache_dir: Option<&Path>,
    output_json: bool,
) -> Result<()> {
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

    let executor = get_executor_from_mode(&config.mode, false);

    if !config.skip_setup {
        start_group!("Preparing the environment");
        executor.setup(&system_info, setup_cache_dir).await?;
        // TODO: refactor and move directly in the Instruments struct as a `setup` method
        if config.instruments.is_mongodb_enabled() {
            install_mongodb_tracer().await?;
        }
        info!("Environment ready");
        end_group!();
    }

    let run_data = get_run_data(&config)?;

    if !config.skip_run {
        start_opened_group!("Running the benchmarks");

        // TODO: refactor and move directly in the Instruments struct as a `start` method
        let mongo_tracer = if let Some(mongodb_config) = &config.instruments.mongodb {
            let mut mongo_tracer = MongoTracer::try_from(&run_data.profile_folder, mongodb_config)?;
            mongo_tracer.start().await?;
            Some(mongo_tracer)
        } else {
            None
        };

        executor
            .run(&config, &system_info, &run_data, &mongo_tracer)
            .await?;

        // TODO: refactor and move directly in the Instruments struct as a `stop` method
        if let Some(mut mongo_tracer) = mongo_tracer {
            mongo_tracer.stop().await?;
        }
        executor.teardown(&config, &system_info, &run_data).await?;

        logger.persist_log_to_profile_folder(&run_data)?;

        end_group!();
    } else {
        debug!("Skipping the run of the benchmarks");
    };

    if !config.skip_upload {
        if provider.get_run_environment() != RunEnvironment::Local {
            // If relevant, set the OIDC token for authentication
            // Note: OIDC tokens can expire quickly, so we set it just before the upload
            provider.set_oidc_token(&mut config).await?;
        }

        start_group!("Uploading performance data");
        let upload_result =
            uploader::upload(&config, &system_info, &provider, &run_data, executor.name()).await?;
        end_group!();

        if provider.get_run_environment() == RunEnvironment::Local {
            poll_results::poll_results(api_client, &provider, upload_result.run_id, output_json)
                .await?;
            end_group!();
        }
    }

    Ok(())
}
