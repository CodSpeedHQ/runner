use std::{env, fmt::Display};

use crate::prelude::*;

mod executor;
mod helpers;
mod interfaces;
mod valgrind;
mod wall_time;

use anyhow::bail;
use executor::Executor;
use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutorName, RunData};
use valgrind::executor::ValgrindExecutor;
use wall_time::executor::WallTimeExecutor;

pub enum RunnerMode {
    Instrumentation,
    WallTime,
}

impl Display for RunnerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerMode::Instrumentation => write!(f, "instrumentation"),
            RunnerMode::WallTime => write!(f, "walltime"),
        }
    }
}

impl TryFrom<&str> for RunnerMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "instrumentation" => Ok(RunnerMode::Instrumentation),
            "walltime" => Ok(RunnerMode::WallTime),
            _ => bail!("Unknown runner mode: {}", value),
        }
    }
}

pub const EXECUTOR_TARGET: &str = "executor";

pub fn get_mode() -> Result<RunnerMode> {
    if let Ok(runner_mode) = env::var("CODSPEED_RUNNER_MODE") {
        debug!("CODSPEED_RUNNER_MODE is set to {}", runner_mode);
        RunnerMode::try_from(runner_mode.as_str())
    } else {
        debug!("CODSPEED_RUNNER_MODE is not set, using instrumentation");
        Ok(RunnerMode::Instrumentation)
    }
}

pub fn get_executor_from_mode(mode: RunnerMode) -> Box<dyn Executor> {
    match mode {
        RunnerMode::Instrumentation => Box::new(ValgrindExecutor),
        RunnerMode::WallTime => Box::new(WallTimeExecutor),
    }
}

pub fn get_all_executors() -> Vec<Box<dyn Executor>> {
    vec![Box::new(ValgrindExecutor), Box::new(WallTimeExecutor)]
}

pub fn get_run_data() -> Result<RunData> {
    let profile_folder = create_profile_folder()?;
    Ok(RunData { profile_folder })
}
