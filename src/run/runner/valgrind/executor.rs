use async_trait::async_trait;

use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};

use super::setup::install_valgrind;
use super::{helpers::perf_maps::harvest_perf_maps, helpers::venv_compat, measure};

pub struct ValgrindExecutor;

#[async_trait(?Send)]
impl Executor for ValgrindExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::Valgrind
    }

    async fn setup(&self, system_info: &SystemInfo) -> Result<()> {
        install_valgrind(system_info).await?;

        if let Err(error) = venv_compat::symlink_libpython(None) {
            error!("Failed to symlink libpython: {error}");
        } else {
            info!("Successfully added symlink for libpython in the venv");
        }

        Ok(())
    }

    async fn run(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
        mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        //TODO: add valgrind version check
        measure::measure(config, &run_data.profile_folder, mongo_tracer).await?;

        Ok(())
    }

    async fn teardown(
        &self,
        _config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        harvest_perf_maps(&run_data.profile_folder).await?;

        Ok(())
    }
}
