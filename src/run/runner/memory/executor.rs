use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::command::CommandBuilder;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::run::runner::helpers::run_with_sudo::wrap_with_sudo;
use crate::run::runner::shared::fifo::RunnerFifo;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use runner_shared::fifo::Command as FifoCommand;
use runner_shared::fifo::RunnerMode as FifoRunnerMode;
use std::path::Path;

pub struct MemoryExecutor;

impl MemoryExecutor {
    fn build_heaptrack_command(config: &Config, run_data: &RunData) -> Result<CommandBuilder> {
        // TODO: Introspected golang/node.js

        let allocations_file = run_data.profile_folder.join("allocations.jsonl");

        // FIXME: Don't  require this to be passed
        let ld_library_path = "/nix/store/pgsgciqx8gn40xa51v6v7jnxs80fs8h9-elfutils-0.192/lib:/nix/store/8icpg7vrz95c6ap3mznmlmg7h0l2av1w-zlib-1.3.1/lib:/nix/store/gj0xrj7ispg9fkbv8igkf5b6z6i80d79-libbpf-1.5.0/lib";

        let mut cmd_builder = CommandBuilder::new("env");
        cmd_builder.arg(format!("LD_LIBRARY_PATH={ld_library_path}"));
        cmd_builder.arg("codspeed-heaptrack");
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

        let runner_fifo = RunnerFifo::new()?;
        let on_process_started = async |_| -> anyhow::Result<()> {
            let data = Self::handle_fifo(runner_fifo).await?;
            // TODO: Figure out how to upload the data to the server
            Ok(())
        };

        let status = run_command_with_log_pipe_and_callback(cmd, on_process_started).await?;
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

impl MemoryExecutor {
    async fn handle_fifo(mut runner_fifo: RunnerFifo) -> anyhow::Result<()> {
        let health_check = async || Ok(true);

        let on_cmd = async |cmd: &FifoCommand| {
            match cmd {
                FifoCommand::StartBenchmark => {
                    // TODO: enable heaptrack
                }
                FifoCommand::StopBenchmark => {
                    // TODO: disable heaptrack
                }
                FifoCommand::GetRunnerMode => {
                    return Ok(FifoCommand::RunnerModeResponse(FifoRunnerMode::Analysis));
                }
                _ => {
                    warn!("Unhandled FIFO command: {cmd:?}");
                    return Ok(FifoCommand::Err);
                }
            }

            Ok(FifoCommand::Ack)
        };

        let _data = runner_fifo
            .handle_fifo_messages(health_check, on_cmd)
            .await?;
        todo!()
    }
}
