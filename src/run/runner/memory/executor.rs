use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::command::CommandBuilder;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::run::runner::helpers::run_with_sudo::wrap_with_sudo;
use crate::run::runner::shared::fifo::RunnerFifo;
use crate::run::runner::{ExecutorName, RunData, RunnerMode};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use ipc_channel::ipc;
use memtrack::MemtrackIpcClient;
use memtrack::MemtrackIpcServer;
use runner_shared::artifacts::{ArtifactExt, ExecutionTimestamps};
use runner_shared::fifo::Command as FifoCommand;
use runner_shared::fifo::IntegrationMode;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use tempfile::NamedTempFile;

fn get_exported_system_env() -> Result<String> {
    let output = Command::new("bash")
        .arg("-c")
        .arg("export")
        .output()
        .context("Failed to run `export`")?;
    if !output.status.success() {
        bail!(
            "Failed to get system environment variables: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).context("Failed to parse export output as UTF-8")
}

pub struct MemoryExecutor;

impl MemoryExecutor {
    fn build_memtrack_command(
        config: &Config,
        run_data: &RunData,
    ) -> Result<(MemtrackIpcServer, CommandBuilder, NamedTempFile)> {
        // FIXME: We only support native languages for now

        // Find memtrack binary - check env variable or use default command name
        let memtrack_path = std::env::var("CODSPEED_MEMTRACK_BINARY")
            .unwrap_or_else(|_| "codspeed-memtrack".to_string());

        // Build the memtrack command
        let memtrack_cmd = format!(
            "{} track {} --output {} --ipc-server {{}}",
            memtrack_path,
            get_bench_command(config)?,
            run_data.profile_folder.join("results").display()
        );

        // Setup memtrack IPC server
        let (ipc_server, server_name) = ipc::IpcOneShotServer::new()?;
        let memtrack_cmd = memtrack_cmd.replace("{}", &server_name);

        // Get system environment variables
        let system_env = get_exported_system_env()?;

        // Get injected environment variables
        let base_injected_env = get_base_injected_env(RunnerMode::Memory, &run_data.profile_folder)
            .into_iter()
            .map(|(k, v)| format!("export {k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");

        // Create environment file
        let combined_env = format!("{system_env}\n{base_injected_env}");
        let mut env_file = NamedTempFile::new()?;
        env_file.write_all(combined_env.as_bytes())?;

        // Create bash command that sources the env file and runs memtrack
        let bash_command = format!("source {} && {}", env_file.path().display(), memtrack_cmd);

        let mut cmd_builder = CommandBuilder::new("bash");
        cmd_builder.arg("-c");
        cmd_builder.arg(bash_command);

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
        // Validate that the codspeed-memtrack command is available
        let memtrack_path = std::env::var("CODSPEED_MEMTRACK_BINARY")
            .unwrap_or_else(|_| "codspeed-memtrack".to_string());

        info!("Validating memtrack binary at path: {memtrack_path}");
        let output = Command::new(&memtrack_path).arg("--version").output()?;
        if !output.status.success() {
            bail!(
                "codspeed-memtrack command is not available or failed to execute\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
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
        // Create the results/ directory inside the profile folder to avoid having memtrack create it with wrong permissions
        std::fs::create_dir_all(run_data.profile_folder.join("results"))?;

        let (ipc, cmd_builder, _env_file) = Self::build_memtrack_command(config, run_data)?;
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
        debug!("cmd exit status: {status:?}");

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
        ipc: MemtrackIpcServer,
    ) -> anyhow::Result<ExecutionTimestamps> {
        debug!("handle_fifo called with PID {pid}");

        // Accept the IPC connection from memtrack and get the sender it sends us
        let (_, memtrack_sender) = ipc.accept()?;
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
