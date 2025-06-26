use super::perf::PerfRunner;
use crate::prelude::*;
use crate::run::RunnerMode;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::{ExecutorName, RunData};
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

    fn walltime_bench_cmd(config: &Config, run_data: &RunData) -> Result<String> {
        let bench_cmd = get_bench_command(config)?;

        let setenv = get_base_injected_env(RunnerMode::Walltime, &run_data.profile_folder)
            .into_iter()
            .map(|(env, value)| format!("--setenv={env}={value}"))
            .join(" ");
        let uid = nix::unistd::Uid::current().as_raw();
        let gid = nix::unistd::Gid::current().as_raw();
        Ok(format!(
            "systemd-run --scope --slice=codspeed.slice --same-dir --uid={uid} --gid={gid} {setenv} -- {bench_cmd}"
        ))
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
        let mut cmd = Command::new("sudo");

        if let Some(cwd) = &config.working_directory {
            let abs_cwd = canonicalize(cwd)?;
            cmd.current_dir(abs_cwd);
        }

        let bench_cmd = Self::walltime_bench_cmd(config, run_data)?;

        let status = if let Some(perf) = &self.perf
            && config.enable_perf
        {
            perf.run(cmd, &bench_cmd, config).await
        } else {
            cmd.args(["sh", "-c", &bench_cmd]);
            debug!("cmd: {cmd:?}");

            run_command_with_log_pipe(cmd).await
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
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        debug!("Copying files to the profile folder");

        if let Some(perf) = &self.perf
            && config.enable_perf
        {
            perf.save_files_to(&run_data.profile_folder).await?;
        }

        Ok(())
    }
}
