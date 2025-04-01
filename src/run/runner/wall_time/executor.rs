use super::perf::PerfFifo;
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::{
    run_command_with_log_pipe, run_command_with_log_pipe_and_callback,
};
use crate::run::runner::{ExecutorName, RunData, RunnerMode};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use codspeed::fifo::FifoIpc;
use codspeed::fifo::RUNNER_ACK_FIFO;
use codspeed::fifo::RUNNER_CTL_FIFO;
use std::fs::canonicalize;
use std::process::Command;
use tempfile::TempDir;

const PERF_DATA_PREFIX: &str = "perf.data.";

pub struct WallTimeExecutor {
    perf_dir: TempDir,
}

impl WallTimeExecutor {
    pub fn new() -> Self {
        Self {
            perf_dir: tempfile::tempdir().unwrap(),
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

        let use_perf = std::env::var("USE_PERF").map(|v| v == "1").unwrap_or(true);
        let status = if use_perf {
            let mut perf_fifo = PerfFifo::new()?;
            let mut ctl_fifo = FifoIpc::create(RUNNER_CTL_FIFO)?.with_reader()?;

            // We have to pass a file to perf, which will create `perf.data.<timestamp>` files
            // when the output is split.
            let perf_file = tempfile::Builder::new()
                .keep(true)
                .prefix(PERF_DATA_PREFIX)
                .tempfile_in(&self.perf_dir)?;

            cmd.args([
                "-c",
                &format!(
                    "perf record --data --freq=1000 --switch-output --control=fifo:{},{} --delay=-1 -g --call-graph=dwarf --output={} -- {}",
                    perf_fifo.ctl_fifo_path.to_string_lossy(),
                    perf_fifo.ack_fifo_path.to_string_lossy(),
                    perf_file.path().to_string_lossy(),
                    get_bench_command(config)?.as_str()
                ),
            ]);
            debug!("cmd: {:?}", cmd);

            let on_process_started = |perf_pid: u32| -> anyhow::Result<()> {
                use codspeed::fifo::Command as FifoCommand;

                let mut ack_fifo = FifoIpc::create(RUNNER_ACK_FIFO)?
                    .with_reader()? // FIFO needs a reader to be opened with writer
                    .with_writer()?;

                debug!("Perf PID: {}", perf_pid);
                std::thread::spawn(move || -> anyhow::Result<()> {
                    loop {
                        let Ok(cmd) = ctl_fifo.recv_cmd() else {
                            continue;
                        };

                        match cmd {
                            FifoCommand::StartBenchmark => {
                                unsafe { libc::kill(perf_pid as i32, libc::SIGUSR2) };
                                perf_fifo.start_events()?;
                                ack_fifo.send_cmd(FifoCommand::Ack)?;
                            }
                            FifoCommand::StopBenchmark => {
                                perf_fifo.stop_events()?;
                                ack_fifo.send_cmd(FifoCommand::Ack)?;
                            }
                            FifoCommand::Ack => unreachable!(),
                        }

                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                });

                Ok(())
            };

            run_command_with_log_pipe_and_callback(cmd, on_process_started)
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
        // Copy the perf data files to the profile folder
        let map_files = std::fs::read_dir(&self.perf_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().starts_with(PERF_DATA_PREFIX))
                    .unwrap_or(false)
            });
        for entry in map_files {
            let perf_map = perf_helper::perf_map::SyntheticPerfMap::from_perf_file(entry.as_path());
            let _ = perf_map.save_to(&run_data.profile_folder);

            if let Some(data) =
                perf_helper::debug_symbols::DebugData::from_perf_file(entry.as_path())
            {
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

        Ok(())
    }
}
