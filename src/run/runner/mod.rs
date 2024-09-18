use std::env;

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
use valgrind::executor::{ValgrindExecutor, INSTRUMENTATION_RUNNER_MODE};
use wall_time::executor::{WallTimeExecutor, WALL_TIME_RUNNER_MODE};

pub const EXECUTOR_TARGET: &str = "executor";

pub fn get_executor() -> Result<Box<dyn Executor>> {
    if let Ok(runner_mode) = env::var("CODSPEED_RUNNER_MODE") {
        debug!("CODSPEED_RUNNER_MODE is set to {}", runner_mode);
        match runner_mode.as_str() {
            INSTRUMENTATION_RUNNER_MODE => Ok(Box::new(ValgrindExecutor)),
            WALL_TIME_RUNNER_MODE => Ok(Box::new(WallTimeExecutor)),
            _ => bail!("Unknown codspeed runner mode"),
        }
    } else {
        debug!("CODSPEED_RUNNER_MODE is not set, using valgrind");
        Ok(Box::new(ValgrindExecutor))
    }
}

pub fn get_run_data() -> Result<RunData> {
    let profile_folder = create_profile_folder()?;
    Ok(RunData { profile_folder })
}
