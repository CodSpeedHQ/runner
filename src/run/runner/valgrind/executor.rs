use async_trait::async_trait;

use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};

use super::{helpers::perf_maps::harvest_perf_maps, measure, setup::setup};

pub struct ValgrindExecutor;

#[async_trait(?Send)]
impl Executor for ValgrindExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::Valgrind
    }

    async fn setup(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        _run_data: &RunData,
    ) -> Result<()> {
        setup(system_info, config).await?;

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
        measure::measure(config, &run_data.profile_folder, mongo_tracer)?;

        Ok(())
    }

    async fn teardown(
        &self,
        _config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        harvest_perf_maps(&run_data.profile_folder)?;

        Ok(())
    }
}
