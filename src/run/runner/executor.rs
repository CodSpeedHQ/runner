use async_trait::async_trait;

use super::interfaces::{ExecutorName, RunData};
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::{check_system::SystemInfo, config::Config};

#[async_trait(?Send)]
pub trait Executor {
    // TODO: this function will be used in a later commit
    #[allow(dead_code)]
    fn name(&self) -> ExecutorName;

    async fn setup(
        &self,
        config: &Config,
        system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()>;

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
