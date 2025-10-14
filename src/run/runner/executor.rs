use super::interfaces::{ExecutorName, RunData};
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::path::Path;

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
