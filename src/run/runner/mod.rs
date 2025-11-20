use std::fmt::Display;

use crate::prelude::*;

use super::{RunnerMode, config::Config};

mod executor;
mod helpers;
mod interfaces;
mod shared;
#[cfg(test)]
mod tests;
mod valgrind;
mod wall_time;

use executor::Executor;
use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutorName, RunData};
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

pub fn get_run_data(config: &Config) -> Result<RunData> {
    let profile_folder = if let Some(profile_folder) = &config.profile_folder {
        profile_folder.clone()
    } else {
        create_profile_folder()?
    };
    Ok(RunData { profile_folder })
}
