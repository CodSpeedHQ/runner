#![cfg_attr(not(unix), allow(dead_code, unused_mut))]

use crate::executor::Config;
use crate::executor::helpers::command::CommandBuilder;
use crate::executor::helpers::env::is_codspeed_debug_enabled;
use crate::executor::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::executor::helpers::run_with_sudo::run_with_sudo;
use crate::executor::helpers::run_with_sudo::wrap_with_sudo;
use crate::executor::valgrind::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::executor::valgrind::helpers::perf_maps::harvest_perf_maps_for_pids;
use crate::executor::wall_time::perf::debug_info::ProcessDebugInfo;
use crate::executor::wall_time::perf::jit_dump::harvest_perf_jit_for_pids;
use crate::executor::wall_time::perf::perf_executable::get_working_perf_executable;
use crate::prelude::*;
use crate::run::UnwindingMode;
use anyhow::Context;
use fifo::{PerfFifo, RunnerFifo};
use libc::pid_t;
use nix::sys::time::TimeValLike;
use nix::time::clock_gettime;
use parse_perf_file::MemmapRecordsOutput;
use runner_shared::debug_info::ModuleDebugInfo;
use runner_shared::fifo::Command as FifoCommand;
use runner_shared::fifo::MarkerType;
use runner_shared::metadata::PerfMetadata;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::{cell::OnceCell, collections::HashMap, process::ExitStatus};

mod jit_dump;
mod parse_perf_file;
mod setup;

pub mod debug_info;
pub mod elf_helper;
pub mod fifo;
pub mod perf_executable;
pub mod perf_map;
pub mod unwind_data;

const PERF_METADATA_CURRENT_VERSION: u64 = 1;
const PERF_DATA_FILE_NAME: &str = "perf.data";
const PERF_PIPEDATA_FILE_NAME: &str = "perf.pipedata";

pub struct PerfRunner {
    benchmark_data: OnceCell<BenchmarkData>,
    /// Whether to output the perf data to a streamable .pipedata file
    output_pipedata: bool,
}

impl PerfRunner {
    pub async fn setup_environment(
        system_info: &crate::run::check_system::SystemInfo,
        setup_cache_dir: Option<&Path>,
    ) -> anyhow::Result<()> {
        setup::install_perf(system_info, setup_cache_dir).await?;

        let sysctl_read = |name: &str| -> anyhow::Result<i64> {
            let output = std::process::Command::new("sysctl").arg(name).output()?;
            let output = String::from_utf8(output.stdout)?;

            Ok(output
                .split(" = ")
                .last()
                .context("Couldn't find the value in sysctl output")?
                .trim()
                .parse::<i64>()?)
        };

        // Allow access to kernel symbols
        if sysctl_read("kernel.kptr_restrict")? != 0 {
            run_with_sudo("sysctl", ["-w", "kernel.kptr_restrict=0"])?;
        }

        // Allow non-root profiling
        if sysctl_read("kernel.perf_event_paranoid")? != -1 {
            run_with_sudo("sysctl", ["-w", "kernel.perf_event_paranoid=-1"])?;
        }

        Ok(())
    }

    pub fn new(output_pipedata: bool) -> Self {
        Self {
            output_pipedata,
            benchmark_data: OnceCell::new(),
        }
    }

    pub async fn run(
        &self,
        mut cmd_builder: CommandBuilder,
        config: &Config,
        profile_folder: &Path,
    ) -> anyhow::Result<ExitStatus> {
        let perf_fifo = PerfFifo::new()?;
        let runner_fifo = RunnerFifo::new()?;

        // Infer the unwinding mode from the benchmark cmd
        let (cg_mode, stack_size) = if let Some(mode) = config.perf_unwinding_mode {
            (mode, None)
        } else if config.command.contains("cargo") {
            (UnwindingMode::Dwarf, None)
        } else if config.command.contains("pytest")
            || config.command.contains("uv")
            || config.command.contains("python")
        {
            // Max supported stack size is 64KiB, but this will increase the file size by a lot. In
            // order to allow uploads and maintain accuracy, we limit this to 8KiB.
            (UnwindingMode::Dwarf, Some(8 * 1024))
        } else {
            // Default to dwarf unwinding since it works well with most binaries.
            debug!("No call graph mode detected, defaulting to dwarf");
            (UnwindingMode::Dwarf, None)
        };

        let cg_mode = match cg_mode {
            UnwindingMode::FramePointer => "fp",
            UnwindingMode::Dwarf => &format!("dwarf,{}", stack_size.unwrap_or(8192)),
        };
        debug!("Using call graph mode: {cg_mode:?}");

        let working_perf_executable =
            get_working_perf_executable().context("Failed to find a working perf executable")?;
        let mut perf_wrapper_builder = CommandBuilder::new(working_perf_executable);
        perf_wrapper_builder.arg("record");
        if !is_codspeed_debug_enabled() {
            perf_wrapper_builder.arg("--quiet");
        }
        perf_wrapper_builder.args([
            "--timestamp",
            // Required for matching the markers and URIs to the samples.
            "-k",
            "CLOCK_MONOTONIC",
            "--freq=997", // Use a prime number to avoid synchronization with periodic tasks
            "--delay=-1",
            "-g",
            "--user-callchains",
            &format!("--call-graph={cg_mode}"),
            &format!(
                "--control=fifo:{},{}",
                perf_fifo.ctl_fifo_path.to_string_lossy(),
                perf_fifo.ack_fifo_path.to_string_lossy()
            ),
        ]);

        if self.output_pipedata {
            perf_wrapper_builder.args([
                "-o", "-", // forces pipe mode
            ]);
        } else {
            perf_wrapper_builder.args([
                "-o",
                self.get_perf_file_path(profile_folder)
                    .to_string_lossy()
                    .as_ref(),
            ]);
        }

        perf_wrapper_builder.arg("--");
        cmd_builder.wrap_with(perf_wrapper_builder);

        // Output the perf data to the profile folder
        let perf_data_file_path = self.get_perf_file_path(profile_folder);

        let raw_command = format!(
            "set -o pipefail && {} | cat > {}",
            &cmd_builder.as_command_line(),
            perf_data_file_path.to_string_lossy()
        );
        let mut wrapped_builder = CommandBuilder::new("bash");
        wrapped_builder.args(["-c", &raw_command]);

        // IMPORTANT: Preserve the working directory from the original command
        if let Some(cwd) = cmd_builder.get_current_dir() {
            wrapped_builder.current_dir(cwd);
        }

        let cmd = wrap_with_sudo(wrapped_builder)?.build();
        debug!("cmd: {cmd:?}");

        let on_process_started = async |_| -> anyhow::Result<()> {
            let data = Self::handle_fifo(runner_fifo, perf_fifo).await?;
            self.benchmark_data.set(data).unwrap_or_else(|_| {
                error!("Failed to set benchmark data in PerfRunner");
            });
            Ok(())
        };
        run_command_with_log_pipe_and_callback(cmd, on_process_started).await
    }

    pub async fn save_files_to(&self, profile_folder: &Path) -> anyhow::Result<()> {
        let start = std::time::Instant::now();

        let bench_data = self
            .benchmark_data
            .get()
            .expect("Benchmark order is not available");

        // Harvest the perf maps generated by python. This will copy the perf
        // maps from /tmp to the profile folder. We have to write our own perf
        // maps to these files AFTERWARDS, otherwise it'll be overwritten!
        harvest_perf_maps_for_pids(profile_folder, &bench_data.bench_pids).await?;
        harvest_perf_jit_for_pids(profile_folder, &bench_data.bench_pids).await?;

        // Append perf maps, unwind info and other metadata
        if let Err(BenchmarkDataSaveError::MissingIntegration) =
            bench_data.save_to(profile_folder, &self.get_perf_file_path(profile_folder))
        {
            warn!(
                "Perf is enabled, but failed to detect benchmarks. If you wish to disable this warning, set CODSPEED_PERF_ENABLED=false"
            );
            return Ok(());
        }

        let elapsed = start.elapsed();
        debug!("Perf teardown took: {elapsed:?}");
        Ok(())
    }

    async fn handle_fifo(
        mut runner_fifo: RunnerFifo,
        mut perf_fifo: PerfFifo,
    ) -> anyhow::Result<BenchmarkData> {
        let mut bench_order_by_timestamp = Vec::<(u64, String)>::new();
        let mut bench_pids = HashSet::<pid_t>::new();
        let mut markers = Vec::<MarkerType>::new();

        let mut integration = None;
        let mut perf_ping_timeout = 5;

        let current_time = || {
            clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC)
                .unwrap()
                .num_nanoseconds() as u64
        };

        let mut benchmark_started = false;
        loop {
            let perf_ping =
                tokio::time::timeout(Duration::from_secs(perf_ping_timeout), perf_fifo.ping())
                    .await;
            if let Ok(Err(_)) | Err(_) = perf_ping {
                debug!("Failed to ping perf FIFO, ending perf fifo loop");
                break;
            }
            // Perf has started successfully, we can decrease the timeout for future pings
            perf_ping_timeout = 1;

            let result = tokio::time::timeout(Duration::from_secs(5), runner_fifo.recv_cmd()).await;
            let cmd = match result {
                Ok(Ok(cmd)) => cmd,
                Ok(Err(e)) => {
                    warn!("Failed to parse FIFO command: {e}");
                    break;
                }
                Err(_) => continue,
            };
            trace!("Received command: {cmd:?}");

            match cmd {
                FifoCommand::CurrentBenchmark { pid, uri } => {
                    bench_order_by_timestamp.push((current_time(), uri.clone()));
                    bench_pids.insert(pid);

                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::StartBenchmark => {
                    if benchmark_started {
                        warn!("Received duplicate StartBenchmark command, ignoring");
                        runner_fifo.send_cmd(FifoCommand::Ack).await?;
                        continue;
                    }
                    benchmark_started = true;
                    markers.push(MarkerType::SampleStart(current_time()));

                    perf_fifo.start_events().await?;
                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::StopBenchmark => {
                    if !benchmark_started {
                        warn!("Received StopBenchmark command before StartBenchmark, ignoring");
                        runner_fifo.send_cmd(FifoCommand::Ack).await?;
                        continue;
                    }
                    benchmark_started = false;

                    markers.push(MarkerType::SampleEnd(current_time()));

                    perf_fifo.stop_events().await?;
                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::PingPerf => {
                    if perf_fifo.ping().await.is_ok() {
                        runner_fifo.send_cmd(FifoCommand::Ack).await?;
                    } else {
                        runner_fifo.send_cmd(FifoCommand::Err).await?;
                    }
                }
                FifoCommand::SetIntegration { name, version } => {
                    integration = Some((name, version));
                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::AddMarker { marker, .. } => {
                    markers.push(marker);
                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::SetVersion(protocol_version) => {
                    if protocol_version < runner_shared::fifo::CURRENT_PROTOCOL_VERSION {
                        panic!(
                            "Integration is using an incompatible protocol version ({protocol_version} < {}). Please update the integration to the latest version.",
                            runner_shared::fifo::CURRENT_PROTOCOL_VERSION
                        )
                    } else if protocol_version > runner_shared::fifo::CURRENT_PROTOCOL_VERSION {
                        panic!(
                            "Runner is using an incompatible protocol version ({} < {protocol_version}). Please update the runner to the latest version.",
                            runner_shared::fifo::CURRENT_PROTOCOL_VERSION
                        )
                    };

                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                _ => {
                    warn!("Received unexpected command: {cmd:?}");
                    runner_fifo.send_cmd(FifoCommand::Err).await?;
                }
            }
        }

        Ok(BenchmarkData {
            integration,
            uri_by_ts: bench_order_by_timestamp,
            bench_pids,
            markers,
        })
    }

    fn get_perf_file_path<P: AsRef<Path>>(&self, profile_folder: P) -> PathBuf {
        if self.output_pipedata {
            profile_folder.as_ref().join(PERF_PIPEDATA_FILE_NAME)
        } else {
            profile_folder.as_ref().join(PERF_DATA_FILE_NAME)
        }
    }
}

pub struct BenchmarkData {
    /// Name and version of the integration
    integration: Option<(String, String)>,

    uri_by_ts: Vec<(u64, String)>,
    bench_pids: HashSet<pid_t>,
    markers: Vec<MarkerType>,
}

#[derive(Debug)]
pub enum BenchmarkDataSaveError {
    MissingIntegration,
    FailedToParsePerfFile,
}

impl BenchmarkData {
    pub fn save_to<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        perf_file_path: P,
    ) -> Result<(), BenchmarkDataSaveError> {
        debug!("Reading perf data from file for mmap extraction");
        let MemmapRecordsOutput {
            symbols_by_pid,
            unwind_data_by_pid,
        } = parse_perf_file::parse_for_memmap2(perf_file_path).map_err(|e| {
            error!("Failed to parse perf file: {e}");
            BenchmarkDataSaveError::FailedToParsePerfFile
        })?;

        for proc_sym in symbols_by_pid.values() {
            proc_sym.save_to(&path).unwrap();
        }

        // Collect debug info for each process by looking up file/line for symbols
        let mut debug_info_by_pid = HashMap::<i32, Vec<ModuleDebugInfo>>::new();
        for (pid, proc_sym) in &symbols_by_pid {
            debug_info_by_pid
                .entry(*pid)
                .or_default()
                .extend(ProcessDebugInfo::new(proc_sym).modules());
        }

        for (pid, modules) in &unwind_data_by_pid {
            for module in modules {
                module.save_to(&path, *pid).unwrap();
            }
        }

        let metadata = PerfMetadata {
            version: PERF_METADATA_CURRENT_VERSION,
            integration: self
                .integration
                .clone()
                .ok_or(BenchmarkDataSaveError::MissingIntegration)?,
            uri_by_ts: self.uri_by_ts.clone(),
            ignored_modules: {
                let mut to_ignore = vec![];

                // Check if any of the ignored modules has been loaded in the process
                for ignore_path in get_objects_path_to_ignore() {
                    for proc in symbols_by_pid.values() {
                        if let Some(mapping) = proc.module_mapping(&ignore_path) {
                            let (Some((base_addr, _)), Some((_, end_addr))) = (
                                mapping.iter().min_by_key(|(base_addr, _)| base_addr),
                                mapping.iter().max_by_key(|(_, end_addr)| end_addr),
                            ) else {
                                continue;
                            };

                            to_ignore.push((ignore_path.clone(), *base_addr, *end_addr));
                        }
                    }
                }

                // When python is statically linked, we'll not find it in the ignored modules. Add it manually:
                let python_modules = symbols_by_pid.values().filter_map(|proc| {
                    proc.loaded_modules().find(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().starts_with("python"))
                            .unwrap_or(false)
                    })
                });
                for path in python_modules {
                    if let Some(mapping) = symbols_by_pid
                        .values()
                        .find_map(|proc| proc.module_mapping(path))
                    {
                        let (Some((base_addr, _)), Some((_, end_addr))) = (
                            mapping.iter().min_by_key(|(base_addr, _)| base_addr),
                            mapping.iter().max_by_key(|(_, end_addr)| end_addr),
                        ) else {
                            continue;
                        };
                        to_ignore.push((path.to_string_lossy().into(), *base_addr, *end_addr));
                    }
                }

                to_ignore
            },
            markers: self.markers.clone(),
            debug_info_by_pid,
        };
        metadata.save_to(&path).unwrap();

        Ok(())
    }
}
