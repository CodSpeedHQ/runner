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

pub struct WallTimeExecutor;

#[async_trait(?Send)]
impl Executor for WallTimeExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::WallTime
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

        cmd.args(["-c", get_bench_command(config)?.as_str()]);

        debug!("cmd: {:?}", cmd);
        let status = run_command_with_log_pipe(cmd)
            .map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?;
        if !status.success() {
            bail!("failed to execute the benchmark process");
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
