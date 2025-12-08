use std::fmt::Display;

pub mod config;
mod execution_context;
mod helpers;
mod interfaces;
#[cfg(test)]
mod tests;
mod valgrind;
mod wall_time;

use crate::instruments::mongo_tracer::{MongoTracer, install_mongodb_tracer};
use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::runner_mode::RunnerMode;
use async_trait::async_trait;
pub use config::Config;
pub use execution_context::ExecutionContext;
pub use helpers::profile_folder::create_profile_folder;
pub use interfaces::ExecutorName;
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

pub fn get_executor_from_mode(mode: &RunnerMode) -> Box<dyn Executor> {
    match mode {
        #[allow(deprecated)]
        RunnerMode::Instrumentation | RunnerMode::Simulation => Box::new(ValgrindExecutor),
        RunnerMode::Walltime => Box::new(WallTimeExecutor::new()),
    }
}

pub fn get_all_executors() -> Vec<Box<dyn Executor>> {
    vec![
        Box::new(ValgrindExecutor),
        Box::new(WallTimeExecutor::new()),
    ]
}

#[async_trait(?Send)]
pub trait Executor {
    fn name(&self) -> ExecutorName;

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
        execution_context: &ExecutionContext,
        // TODO: use Instruments instead of directly passing the mongodb tracer
        mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()>;

    async fn teardown(&self, execution_context: &ExecutionContext) -> Result<()>;
}

/// Execute benchmarks with the given configuration
/// This is the core execution logic shared between `run` and `exec` commands
pub async fn execute_benchmarks(
    executor: &Box<dyn Executor>,
    execution_context: &mut ExecutionContext,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    if !execution_context.config.skip_setup {
        start_group!("Preparing the environment");
        executor
            .setup(&execution_context.system_info, setup_cache_dir)
            .await?;
        // TODO: refactor and move directly in the Instruments struct as a `setup` method
        if execution_context.config.instruments.is_mongodb_enabled() {
            install_mongodb_tracer().await?;
        }
        info!("Environment ready");
        end_group!();
    }

    if !execution_context.config.skip_run {
        start_opened_group!("Running the benchmarks");

        // TODO: refactor and move directly in the Instruments struct as a `start` method
        let mongo_tracer =
            if let Some(mongodb_config) = &execution_context.config.instruments.mongodb {
                let mut mongo_tracer =
                    MongoTracer::try_from(&execution_context.profile_folder, mongodb_config)?;
                mongo_tracer.start().await?;
                Some(mongo_tracer)
            } else {
                None
            };

        executor.run(execution_context, &mongo_tracer).await?;

        // TODO: refactor and move directly in the Instruments struct as a `stop` method
        if let Some(mut mongo_tracer) = mongo_tracer {
            mongo_tracer.stop().await?;
        }
        executor.teardown(execution_context).await?;

        execution_context
            .logger
            .persist_log_to_profile_folder(execution_context)?;

        end_group!();
    } else {
        debug!("Skipping the run of the benchmarks");
    };

    Ok(())
}
