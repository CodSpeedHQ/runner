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
use heaptrack::HeaptrackIpcClient;
use heaptrack::HeaptrackIpcServer;
use ipc_channel::ipc;
use runner_shared::benchmark_results::{BenchmarkResultExt, MarkerResult};
use runner_shared::fifo::Command as FifoCommand;
use runner_shared::fifo::IntegrationMode;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

pub struct MemoryExecutor;

impl MemoryExecutor {
    fn build_heaptrack_command(
        config: &Config,
        run_data: &RunData,
    ) -> Result<(HeaptrackIpcServer, CommandBuilder)> {
        // FIXME: We only support native languages for now

        // Find heaptrack binary - check env variable or use default command name
        let heaptrack_path = std::env::var("CODSPEED_HEAPTRACK_BINARY")
            .unwrap_or_else(|_| "codspeed-heaptrack".to_string());

        // Always use env to preserve LD_LIBRARY_PATH and other environment variables
        let mut cmd_builder = CommandBuilder::new("env");

        // Preserve LD_LIBRARY_PATH from the current environment if it exists
        if let Ok(ld_library_path) = std::env::var("LD_LIBRARY_PATH") {
            cmd_builder.arg(format!("LD_LIBRARY_PATH={ld_library_path}"));
        }

        cmd_builder.arg(&heaptrack_path);
        cmd_builder.arg("track");
        cmd_builder.arg(get_bench_command(config)?);
        cmd_builder.arg("--output");
        cmd_builder.arg(run_data.profile_folder.join("results"));

        // Setup heaptrack IPC server
        let (ipc_server, server_name) = ipc::IpcOneShotServer::new()?;
        cmd_builder.arg("--ipc-server");
        cmd_builder.arg(server_name);

        Ok((ipc_server, cmd_builder))
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
        // Validate that the codspeed-heaptrack command is available
        let heaptrack_path = std::env::var("CODSPEED_HEAPTRACK_BINARY")
            .unwrap_or_else(|_| "codspeed-heaptrack".to_string());

        info!("Validating heaptrack binary at path: {}", heaptrack_path);
        let output = Command::new(&heaptrack_path).arg("--version").output()?;
        if !output.status.success() {
            bail!("codspeed-heaptrack command is not available or failed to execute");
        }

        Ok(())
    }

    async fn run(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
        _mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        // Create the results/ directory inside the profile folder to avoid having heaptrack create it with wrong permissions
        std::fs::create_dir_all(run_data.profile_folder.join("results"))?;

        let (ipc, cmd_builder) = Self::build_heaptrack_command(config, run_data)?;
        let cmd = wrap_with_sudo(cmd_builder)?.build();
        debug!("cmd: {cmd:?}");

        let runner_fifo = RunnerFifo::new()?;
        let on_process_started = async |pid| -> anyhow::Result<()> {
            let marker_result = Self::handle_fifo(runner_fifo, pid, ipc).await?;

            // Directly write to the profile folder, to avoid having to define another field
            marker_result
                .save_to(run_data.profile_folder.join("results"))
                .unwrap();

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
        Ok(())
    }
}

impl MemoryExecutor {
    async fn handle_fifo(
        mut runner_fifo: RunnerFifo,
        pid: u32,
        ipc: HeaptrackIpcServer,
    ) -> anyhow::Result<MarkerResult> {
        debug!("handle_fifo called with PID {pid}");

        // Accept the IPC connection from heaptrack and get the sender it sends us
        let (_, heaptrack_sender) = ipc.accept()?;
        let ipc_client = Rc::new(HeaptrackIpcClient::from_accepted(heaptrack_sender));

        let ipc_client_health = Rc::clone(&ipc_client);
        let health_check = async move || {
            // Ping heaptrack via IPC to check if it's still responding
            match ipc_client_health.ping() {
                Ok(()) => Ok(true),
                Err(_) => Ok(false),
            }
        };

        let on_cmd = async move |cmd: &FifoCommand| {
            match cmd {
                FifoCommand::StartBenchmark => {
                    debug!("Enabling heaptrack via IPC");
                    if let Err(e) = ipc_client.enable() {
                        error!("Failed to enable heaptrack: {e}");
                        return Ok(FifoCommand::Err);
                    }
                }
                FifoCommand::StopBenchmark => {
                    debug!("Disabling heaptrack via IPC");
                    if let Err(e) = ipc_client.disable() {
                        // There's a chance that heaptrack has already exited here, so just log as debug
                        debug!("Failed to disable heaptrack: {e}");
                        return Ok(FifoCommand::Err);
                    }
                }
                FifoCommand::GetIntegrationMode => {
                    return Ok(FifoCommand::IntegrationModeResponse(
                        IntegrationMode::Analysis,
                    ));
                }
                _ => {
                    warn!("Unhandled FIFO command: {cmd:?}");
                    return Ok(FifoCommand::Err);
                }
            }

            Ok(FifoCommand::Ack)
        };

        let (marker_result, _) = runner_fifo
            .handle_fifo_messages(health_check, on_cmd)
            .await?;
        Ok(marker_result)
    }
}
