use crate::binary_installer::ensure_binary_installed;
use crate::executor::ExecutorName;
use crate::executor::helpers::command::CommandBuilder;
use crate::executor::helpers::env::get_base_injected_env;
use crate::executor::helpers::get_bench_command::get_bench_command;
use crate::executor::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::executor::helpers::run_with_env::wrap_with_env;
use crate::executor::helpers::run_with_sudo::wrap_with_sudo;
use crate::executor::shared::fifo::RunnerFifo;
use crate::executor::{ExecutionContext, Executor};
use crate::instruments::mongo_tracer::MongoTracer;
use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::runner_mode::RunnerMode;
use async_trait::async_trait;
use ipc_channel::ipc;
use memtrack::MemtrackIpcClient;
use memtrack::MemtrackIpcServer;
use runner_shared::artifacts::{ArtifactExt, ExecutionTimestamps};
use runner_shared::fifo::Command as FifoCommand;
use runner_shared::fifo::IntegrationMode;
use std::path::Path;
use std::rc::Rc;
use tempfile::NamedTempFile;
use tokio::time::{Duration, timeout};

const MEMTRACK_COMMAND: &str = "codspeed-memtrack";
const MEMTRACK_CODSPEED_VERSION: &str = "1.0.0";

pub struct MemoryExecutor;

impl MemoryExecutor {
    fn build_memtrack_command(
        execution_context: &ExecutionContext,
    ) -> Result<(MemtrackIpcServer, CommandBuilder, NamedTempFile)> {
        // FIXME: We only support native languages for now

        // Setup memtrack IPC server
        let (ipc_server, server_name) = ipc::IpcOneShotServer::new()?;

        // Build the memtrack command
        let mut cmd_builder = CommandBuilder::new(MEMTRACK_COMMAND);
        cmd_builder.arg("track");
        cmd_builder.arg("--output");
        cmd_builder.arg(execution_context.profile_folder.join("results"));
        cmd_builder.arg("--ipc-server");
        cmd_builder.arg(server_name);
        cmd_builder.arg(get_bench_command(&execution_context.config)?);

        // Wrap command with environment forwarding
        let extra_env =
            get_base_injected_env(RunnerMode::Memory, &execution_context.profile_folder);
        let (cmd_builder, env_file) = wrap_with_env(cmd_builder, &extra_env)?;

        Ok((ipc_server, cmd_builder, env_file))
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
        let get_memtrack_installer_url = || {
            format!(
                "https://github.com/CodSpeedHQ/runner/releases/download/memtrack-v{MEMTRACK_CODSPEED_VERSION}/memtrack-installer.sh"
            )
        };

        ensure_binary_installed(
            MEMTRACK_COMMAND,
            MEMTRACK_CODSPEED_VERSION,
            get_memtrack_installer_url,
        )
        .await?;

        Ok(())
    }

    async fn run(
        &self,
        execution_context: &ExecutionContext,
        _mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        // Create the results/ directory inside the profile folder to avoid having memtrack create it with wrong permissions
        std::fs::create_dir_all(execution_context.profile_folder.join("results"))?;

        let (ipc, cmd_builder, _env_file) = Self::build_memtrack_command(execution_context)?;
        let cmd = wrap_with_sudo(cmd_builder)?.build();
        debug!("cmd: {cmd:?}");

        let runner_fifo = RunnerFifo::new()?;
        let on_process_started = async |pid| -> anyhow::Result<()> {
            let marker_result = Self::handle_fifo(runner_fifo, pid, ipc).await?;

            // Directly write to the profile folder, to avoid having to define another field
            marker_result
                .save_to(execution_context.profile_folder.join("results"))
                .unwrap();

            Ok(())
        };

        let status = run_command_with_log_pipe_and_callback(cmd, on_process_started).await?;
        debug!("cmd exit status: {status:?}");

        if !status.success() {
            bail!("failed to execute memory tracker process: {status}");
        }

        Ok(())
    }

    async fn teardown(&self, _execution_context: &ExecutionContext) -> Result<()> {
        Ok(())
    }
}

impl MemoryExecutor {
    async fn handle_fifo(
        mut runner_fifo: RunnerFifo,
        pid: u32,
        ipc: MemtrackIpcServer,
    ) -> anyhow::Result<ExecutionTimestamps> {
        debug!("handle_fifo called with PID {pid}");

        // Accept the IPC connection from memtrack and get the sender it sends us
        // Use a timeout to prevent hanging if the process doesn't start properly
        // https://github.com/servo/ipc-channel/issues/261
        let (_, memtrack_sender) = timeout(Duration::from_secs(5), async move {
            tokio::task::spawn_blocking(move || ipc.accept())
                .await
                .context("Failed to spawn blocking task")?
                .context("Failed to accept IPC connection")
        })
        .await
        .context("Timeout waiting for IPC connection from memtrack process")??;
        let ipc_client = Rc::new(MemtrackIpcClient::from_accepted(memtrack_sender));

        let ipc_client_health = Rc::clone(&ipc_client);
        let health_check = async move || {
            // Ping memtrack via IPC to check if it's still responding
            match ipc_client_health.ping() {
                Ok(()) => Ok(true),
                Err(_) => Ok(false),
            }
        };

        let on_cmd = async move |cmd: &FifoCommand| {
            match cmd {
                FifoCommand::CurrentBenchmark { pid, uri } => {
                    debug!("Current benchmark: {pid}, {uri}");
                }
                FifoCommand::StartBenchmark => {
                    debug!("Enabling memtrack via IPC");
                    if let Err(e) = ipc_client.enable() {
                        error!("Failed to enable memtrack: {e}");
                        return Ok(FifoCommand::Err);
                    }
                }
                FifoCommand::StopBenchmark => {
                    debug!("Disabling memtrack via IPC");
                    if let Err(e) = ipc_client.disable() {
                        // There's a chance that memtrack has already exited here, so just log as debug
                        debug!("Failed to disable memtrack: {e}");
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
