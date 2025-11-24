use std::fmt::Display;

mod helpers;
mod interfaces;
mod memory;
mod shared;
#[cfg(test)]
mod tests;
mod valgrind;
mod wall_time;

use crate::instruments::mongo_tracer::MongoTracer;
use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::run::config::Config;
use crate::runner_mode::RunnerMode;
use async_trait::async_trait;
use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutorName, RunData};
use memory::executor::MemoryExecutor;
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
            RunnerMode::Memory => write!(f, "memory"),
        }
    }
}

pub const EXECUTOR_TARGET: &str = "executor";

pub fn get_executor_from_mode(mode: &RunnerMode) -> Box<dyn Executor> {
    match mode {
        #[allow(deprecated)]
        RunnerMode::Instrumentation | RunnerMode::Simulation => Box::new(ValgrindExecutor),
        RunnerMode::Walltime => Box::new(WallTimeExecutor::new()),
        RunnerMode::Memory => Box::new(MemoryExecutor),
    }
}

pub fn get_all_executors() -> Vec<Box<dyn Executor>> {
    vec![
        Box::new(ValgrindExecutor),
        Box::new(WallTimeExecutor::new()),
        Box::new(MemoryExecutor),
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
