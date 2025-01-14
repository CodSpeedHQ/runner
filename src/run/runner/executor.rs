use super::helpers::env::BASE_INJECTED_ENV;
use super::interfaces::{ExecutorName, RunData};
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

#[async_trait(?Send)]
pub trait Executor {
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

    /// Gets the base environment for the command
    ///
    /// Later on, we will want to refactor this and create the cmd directly in a trait function
    fn get_cmd_base_envs(&self, profile_folder: &Path) -> HashMap<&str, String> {
        let mut hashmap = BASE_INJECTED_ENV.clone();
        hashmap.insert("CODSPEED_RUNNER_MODE", self.name().to_string());
        hashmap.insert(
            "CODSPEED_PROFILE_FOLDER",
            profile_folder.to_str().unwrap().to_string(),
        );
        hashmap
    }
}
