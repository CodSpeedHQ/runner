use super::helpers::validate_memory_results;
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::command::CommandBuilder;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::helpers::run_with_sudo::wrap_with_sudo;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::path::Path;

pub struct MemoryExecutor;

impl MemoryExecutor {
    fn build_heaptrack_command(config: &Config, run_data: &RunData) -> Result<CommandBuilder> {
        let heaptrack_binary = std::env::var("CODSPEED_HEAPTRACK_BINARY").unwrap();
        let allocations_file = run_data.profile_folder.join("allocations.jsonl");

        let mut cmd_builder = CommandBuilder::new(heaptrack_binary);
        cmd_builder.arg("track");
        cmd_builder.arg(get_bench_command(config)?);
        cmd_builder.arg("--output");
        cmd_builder.arg(allocations_file);

        Ok(cmd_builder)
    }
}

#[async_trait(?Send)]
impl Executor for MemoryExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::Memory
    }

    async fn setup(
        &self,
        _system_info: &SystemInfo,
        _setup_cache_dir: Option<&Path>,
    ) -> Result<()> {
        // TODO: Validate that we have the binary + rights to run it
        Ok(())
    }

    async fn run(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
        _mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        let cmd_builder = Self::build_heaptrack_command(config, run_data)?;
        let cmd = wrap_with_sudo(cmd_builder)?.build();
        debug!("cmd: {cmd:?}");

        let status = run_command_with_log_pipe(cmd)
            .await
            .map_err(|e| anyhow!("failed to execute the benchmark process. {e}"))?;
        debug!("cmd exit status: {:?}", status);

        if !status.success() {
            bail!("failed to execute memory tracker process: {status}");
        }

        Ok(())
    }

    async fn teardown(
        &self,
        _config: &Config,
        _system_info: &SystemInfo,
        _run_data: &RunData,
    ) -> Result<()> {
        // TODO: Copy the results to the profile folder
        Ok(())
    }
}
