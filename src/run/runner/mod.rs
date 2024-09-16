use std::env;

use crate::prelude::*;

mod executor;
mod helpers;
mod interfaces;
mod valgrind;

use anyhow::bail;
use executor::Executor;
use helpers::profile_folder::create_profile_folder;
pub use interfaces::{ExecutorName, RunData};
use valgrind::executor::{ValgrindExecutor, INSTRUMENTATION_RUNNER_MODE};

pub use valgrind::VALGRIND_EXECUTION_TARGET;

pub fn get_executor() -> Result<Box<dyn Executor>> {
    if let Ok(runner_mode) = env::var("CODSPEED_RUNNER_MODE") {
        debug!("CODSPEED_RUNNER_MODE is set to {}", runner_mode);
        match runner_mode.as_str() {
            INSTRUMENTATION_RUNNER_MODE => Ok(Box::new(ValgrindExecutor)),
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
