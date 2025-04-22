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
        let use_perf = std::env::var("USE_PERF").map(|v| v == "1").unwrap_or(true);
        debug!("Running the cmd with perf: {}", use_perf);

        Self {
            perf: use_perf.then(PerfRunner::new),
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
        let mut cmd = Command::new("sh");

        cmd.envs(get_base_injected_env(
            RunnerMode::Walltime,
            &run_data.profile_folder,
        ));

        if let Some(cwd) = &config.working_directory {
            let abs_cwd = canonicalize(cwd)?;
            cmd.current_dir(abs_cwd);
        }

        let bench_cmd = get_bench_command(config)?;
        let status = if let Some(perf) = &self.perf {
            perf.run(cmd, &bench_cmd).await
        } else {
            cmd.args(["-c", &bench_cmd]);
            debug!("cmd: {:?}", cmd);

            run_command_with_log_pipe(cmd).await
        };

        if !status
            .map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?
            .success()
        {
            bail!("failed to execute the benchmark process");
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
