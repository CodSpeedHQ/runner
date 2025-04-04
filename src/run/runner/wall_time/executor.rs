use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::{
    run_command_with_log_pipe, run_command_with_log_pipe_and_callback,
};
use crate::run::runner::wall_time::perf;
use crate::run::runner::wall_time::perf::fifo::PerfFifo;
use crate::run::runner::wall_time::perf::fifo::RunnerFifo;
use crate::run::runner::{ExecutorName, RunData, RunnerMode};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::cell::OnceCell;
use std::fs::canonicalize;
use std::process::Command;
use tempfile::TempDir;

use super::perf::perf_map::SyntheticPerfMap;
use super::perf::unwind_data::UnwindDataLoader;

const PERF_DATA_PREFIX: &str = "perf.data.";

pub struct WallTimeExecutor {
    use_perf: bool,
    perf_dir: TempDir,
    bench_order: OnceCell<Vec<String>>,
}

impl WallTimeExecutor {
    pub fn new() -> Self {
        let use_perf = std::env::var("USE_PERF").map(|v| v == "1").unwrap_or(true);
        debug!("Running the cmd with perf: {}", use_perf);

        Self {
            use_perf,
            perf_dir: tempfile::tempdir().expect("Failed to create temporary directory"),
            bench_order: OnceCell::new(),
        }
    }
}

#[async_trait(?Send)]
impl Executor for WallTimeExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::WallTime
    }

    async fn setup(&self, _system_info: &SystemInfo) -> Result<()> {
        super::perf::setup_environment()
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

        let status = if self.use_perf {
            let perf_fifo = PerfFifo::new()?;
            let runner_fifo = RunnerFifo::new()?;

            // We have to pass a file to perf, which will create `perf.data.<timestamp>` files
            // when the output is split.
            let perf_file = tempfile::Builder::new()
                .keep(true)
                .prefix(PERF_DATA_PREFIX)
                .tempfile_in(&self.perf_dir)?;

            cmd.args([
                "-c",
                &format!(
                    "perf record --data --freq=max --switch-output --control=fifo:{},{} --delay=-1 -g --call-graph=dwarf --output={} -- {}",
                    perf_fifo.ctl_fifo_path.to_string_lossy(),
                    perf_fifo.ack_fifo_path.to_string_lossy(),
                    perf_file.path().to_string_lossy(),
                    get_bench_command(config)?.as_str()
                ),
            ]);
            debug!("cmd: {:?}", cmd);

            let mut thread_handle = None;
            let on_process_started = |pid: u32| -> anyhow::Result<()> {
                debug!("Process id: {}", pid);

                let handle = tokio::task::spawn(async move {
                    perf::fifo::handle_fifo(pid, runner_fifo, perf_fifo).await
                });
                thread_handle = Some(handle);

                Ok(())
            };
            let status = run_command_with_log_pipe_and_callback(cmd, on_process_started);
            info!("Process finished with status: {:?}", status);

            // Write the bench_order to the perf directory
            let bench_order = thread_handle
                .context("No thread found")?
                .await
                .map_err(|e| anyhow!("failed to join thread: {:?}", e))??;
            let _ = self.bench_order.set(bench_order);

            status
        } else {
            cmd.args(["-c", get_bench_command(config)?.as_str()]);
            run_command_with_log_pipe(cmd)
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

        if self.use_perf {
            let bench_order = self
                .bench_order
                .get()
                .expect("Benchmark order is not available");

            // Copy the perf data files to the profile folder
            let map_files = std::fs::read_dir(&self.perf_dir)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().starts_with(PERF_DATA_PREFIX))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();

            assert_eq!(
                map_files.len() - 1, // First perf.data is empty
                bench_order.len(),
                "Number of perf data files does not match the number of benchmarks"
            );

            for entry in map_files {
                let perf_map = SyntheticPerfMap::from_perf_file(entry.as_path());
                let _ = perf_map.save_to(&run_data.profile_folder);

                if let Some(data) = UnwindDataLoader::from_perf_file(entry.as_path()) {
                    data.save_to(&run_data.profile_folder)?;
                }

                let src_path = &entry;
                let dst_file_name = format!(
                    "{}.perf",
                    entry.file_name().unwrap_or_default().to_string_lossy()
                );
                let dst_path = run_data.profile_folder.join(dst_file_name);
                std::fs::copy(src_path, dst_path)?;
            }

            // Copy bench_order.txt to the profile folder
            std::fs::write(
                run_data.profile_folder.join("metadata.bench_order"),
                bench_order.join("\n"),
            )?;
        }

        Ok(())
    }
}
