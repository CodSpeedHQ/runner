use super::perf::BenchmarkData;
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

const PERF_DATA_PREFIX: &str = "perf.data.";

pub struct WallTimeExecutor {
    use_perf: bool,
    perf_dir: TempDir,
    benchmark_data: OnceCell<BenchmarkData>,
}

impl WallTimeExecutor {
    pub fn new() -> Self {
        let use_perf = std::env::var("USE_PERF").map(|v| v == "1").unwrap_or(true);
        debug!("Running the cmd with perf: {}", use_perf);

        Self {
            use_perf,
            perf_dir: tempfile::tempdir().expect("Failed to create temporary directory"),
            benchmark_data: OnceCell::new(),
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
                    "perf record --user-callchains --freq=max --switch-output --control=fifo:{},{} --delay=-1 -g --call-graph=dwarf --output={} -- {}",
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
            let benchmark_data = thread_handle
                .context("No thread found")?
                .await
                .map_err(|e| anyhow!("failed to join thread: {:?}", e))??;
            let _ = self.benchmark_data.set(benchmark_data);

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
            let bench_data = self
                .benchmark_data
                .get()
                .expect("Benchmark order is not available");
            bench_data.save_to(&run_data.profile_folder).unwrap();

            // Copy the perf data files to the profile folder
            let map_files = std::fs::read_dir(&self.perf_dir)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path().to_path_buf())
                .filter(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().starts_with(PERF_DATA_PREFIX))
                        .unwrap_or(false)
                })
                .sorted_by_key(|path| path.file_name().unwrap().to_string_lossy().to_string())
                // The first perf.data will only contain metadata that is not relevant to the benchmarks. We
                // capture the symbols and unwind data separately.
                .skip(1)
                .collect::<Vec<_>>();

            assert_eq!(
                map_files.len(),
                bench_data.bench_count(),
                "Benchmark count mismatch"
            );

            for entry in map_files {
                let src_path = &entry;
                let dst_file_name = format!(
                    "{}_{}.perf",
                    perf::helpers::find_pid(&entry)?,
                    entry.file_name().unwrap_or_default().to_string_lossy(),
                );
                let dst_path = run_data.profile_folder.join(dst_file_name);
                std::fs::copy(src_path, dst_path)?;
            }
        }

        Ok(())
    }
}
