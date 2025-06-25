use super::perf::PerfRunner;
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::{ExecutorName, RunData, RunnerMode};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::fs::canonicalize;
use std::process::Command;

pub struct WallTimeExecutor {
    perf: Option<PerfRunner>,
}

impl WallTimeExecutor {
    pub fn new() -> Self {
        Self {
            perf: cfg!(target_os = "linux").then(PerfRunner::new),
        }
    }
}

#[async_trait(?Send)]
impl Executor for WallTimeExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::WallTime
    }

    async fn setup(&self, _system_info: &SystemInfo) -> Result<()> {
        if self.perf.is_some() {
            PerfRunner::setup_environment()?;
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
        // IMPORTANT: Don't use `sh` here! We will use this pid to send signals to the
        // spawned child process which won't work if we use a different shell.
        let mut cmd = Command::new("bash");

        cmd.envs(get_base_injected_env(
            RunnerMode::Walltime,
            &run_data.profile_folder,
        ));

        if let Some(cwd) = &config.working_directory {
            let abs_cwd = canonicalize(cwd)?;
            cmd.current_dir(abs_cwd);
        }

        let bench_cmd = get_bench_command(config)?;
        let status = match (config.enable_perf, &self.perf) {
            (true, Some(perf)) => perf.run(cmd, &bench_cmd, config).await,
            _ => {
                cmd.args(["-c", &bench_cmd]);
                debug!("cmd: {:?}", cmd);

                run_command_with_log_pipe(cmd).await
            }
        };

        let status =
            status.map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?;
        debug!("cmd exit status: {:?}", status);

        if !status.success() {
            bail!("failed to execute the benchmark process: {}", status);
        }

        Ok(())
    }

    async fn teardown(
        &self,
        _config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        debug!("Copying files to the profile folder");

        if let Some(perf) = &self.perf {
            perf.save_files_to(&run_data.profile_folder).await?;
        }

        Ok(())
    }
}
