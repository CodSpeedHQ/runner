use super::perf::PerfFifo;
use super::perf::PERF_CTL_ACK_FIFO;
use super::perf::PERF_CTL_FIFO;
use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::run::runner::{ExecutorName, RunData, RunnerMode};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use codspeed::fifo::FifoIpc;
use codspeed::fifo::RUNNER_ACK_FIFO;
use codspeed::fifo::RUNNER_CTL_FIFO;
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
        super::perf::setup_environment();

        let mut cmd = Command::new("sh");
        cmd.envs(get_base_injected_env(
            RunnerMode::Walltime,
            &run_data.profile_folder,
        ));

        if let Some(cwd) = &config.working_directory {
            let abs_cwd = canonicalize(cwd)?;
            cmd.current_dir(abs_cwd);
        }

        // Create the perf FIFOs before starting the command
        let mut perf_fifo = PerfFifo::new();
        let mut ctl_fifo = FifoIpc::create(RUNNER_CTL_FIFO)
            .unwrap()
            .with_reader()
            .unwrap();

        // We have to pass a file to perf, which will create `perf.data.<timestamp>` files
        // when the output is split.
        let perf_dir = tempfile::tempdir()?;
        let perf_file = tempfile::Builder::new()
            .keep(true)
            .prefix("perf.data")
            .tempfile_in(&perf_dir)?;

        cmd.args([
            "-c",
            &format!(
                "perf record --data --freq=1000 --switch-output --control=fifo:{},{} --delay=-1 -g --output={} -- {}",
                PERF_CTL_FIFO,
                PERF_CTL_ACK_FIFO,
                perf_file.path().to_string_lossy(),
                get_bench_command(config)?.as_str()
            ),
        ]);
        debug!("cmd: {:?}", cmd);

        let on_process_started = |perf_pid: u32| {
            use codspeed::fifo::Command as FifoCommand;

            let mut ack_fifo = FifoIpc::create(RUNNER_ACK_FIFO)
                .unwrap()
                .with_reader() // FIFO needs a reader to be opened with writer
                .unwrap()
                .with_writer()
                .unwrap();

            debug!("Perf PID: {}", perf_pid);
            std::thread::spawn(move || loop {
                let Some(cmd) = ctl_fifo.recv_cmd() else {
                    continue;
                };

                match cmd {
                    FifoCommand::StartBenchmark => {
                        unsafe { libc::kill(perf_pid as i32, libc::SIGUSR2) };
                        perf_fifo.start_events();
                        ack_fifo.send_cmd(FifoCommand::Ack).unwrap();
                    }
                    FifoCommand::StopBenchmark => {
                        perf_fifo.stop_events();
                        ack_fifo.send_cmd(FifoCommand::Ack).unwrap();
                    }
                    FifoCommand::Ack => unreachable!(),
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            });
        };
        let status = run_command_with_log_pipe_and_callback(cmd, on_process_started)
            .map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?;
        if !status.success() {
            bail!("failed to execute the benchmark process");
        }

        // Collect the perf.data traces
        let mut perf_files = Vec::new();
        for entry in std::fs::read_dir(&perf_dir)?.filter_map(|entry| entry.ok()) {
            perf_files.push(entry.path());
        }
        dbg!(&perf_files);

        // TODO: Upload the files

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
