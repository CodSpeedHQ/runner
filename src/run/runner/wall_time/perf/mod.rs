#![cfg_attr(not(unix), allow(dead_code, unused_mut))]

use crate::prelude::*;
use crate::run::UnwindingMode;
use crate::run::config::Config;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe_and_callback;
use crate::run::runner::helpers::setup::run_with_sudo;
use crate::run::runner::valgrind::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::run::runner::valgrind::helpers::perf_maps::harvest_perf_maps_for_pids;
use anyhow::Context;
use fifo::{PerfFifo, RunnerFifo};
use futures::stream::FuturesUnordered;
use metadata::PerfMetadata;
use perf_map::ProcessSymbols;
use shared::Command as FifoCommand;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{cell::OnceCell, collections::HashMap, process::ExitStatus};
use tempfile::TempDir;
use unwind_data::UnwindData;

mod metadata;
mod setup;
mod shared;
pub use shared::*;

pub mod fifo;
pub mod helpers;
pub mod perf_map;
pub mod unwind_data;

const PERF_DATA_PREFIX: &str = "perf.data.";

struct EnvGuard {
    post_bench_script: PathBuf,
}

impl EnvGuard {
    fn execute_script_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if !path.exists() || !path.is_file() {
            warn!("Script not found or not a file: {}", path.display());
            return Ok(());
        }

        let output = Command::new("bash").args([&path]).output()?;
        if !output.status.success() {
            info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            bail!("Failed to execute script: {}", path.display());
        }

        Ok(())
    }

    pub fn setup_with_scripts<P: AsRef<Path>>(pre_bench_script: P, post_bench_script: P) -> Self {
        if let Err(e) = Self::execute_script_from_path(pre_bench_script.as_ref()) {
            warn!("Failed to execute pre-bench script: {e}");
            println!("asdf: {e}");
        }

        Self {
            post_bench_script: post_bench_script.as_ref().to_path_buf(),
        }
    }

    pub fn setup() -> Self {
        Self::setup_with_scripts(
            "/usr/local/bin/codspeed-pre-bench",
            "/usr/local/bin/codspeed-post-bench",
        )
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Err(e) = Self::execute_script_from_path(&self.post_bench_script) {
            warn!("Failed to execute post-bench script: {e}");
        }
    }
}

pub struct PerfRunner {
    perf_dir: TempDir,
    benchmark_data: OnceCell<BenchmarkData>,
}

impl PerfRunner {
    pub fn setup_environment() -> anyhow::Result<()> {
        setup::install_perf()?;

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
            run_with_sudo(&["sysctl", "-w", "kernel.kptr_restrict=0"])?;
        }

        // Allow non-root profiling
        if sysctl_read("kernel.perf_event_paranoid")? != -1 {
            run_with_sudo(&["sysctl", "-w", "kernel.perf_event_paranoid=-1"])?;
        }

        Ok(())
    }

    pub fn new() -> Self {
        Self {
            perf_dir: tempfile::tempdir().expect("Failed to create temporary directory"),
            benchmark_data: OnceCell::new(),
        }
    }

    pub async fn run(
        &self,
        mut cmd: Command,
        bench_cmd: &str,
        config: &Config,
    ) -> anyhow::Result<ExitStatus> {
        let perf_fifo = PerfFifo::new()?;
        let runner_fifo = RunnerFifo::new()?;

        // We have to pass a file to perf, which will create `perf.data.<timestamp>` files
        // when the output is split.
        let perf_file = tempfile::Builder::new()
            .keep(true)
            .prefix(PERF_DATA_PREFIX)
            .tempfile_in(&self.perf_dir)?;

        // Infer the unwinding mode from the benchmark cmd
        let cg_mode = match (config.perf_unwinding_mode, &bench_cmd) {
            (Some(mode), _) => mode,
            (None, cmd) if cmd.contains("pytest") => UnwindingMode::FramePointer,
            (None, cmd) if cmd.contains("cargo") => UnwindingMode::Dwarf,
            (None, _) => {
                // Default to dwarf unwinding since it works well with most binaries.
                debug!("No call graph mode detected, defaulting to dwarf");
                UnwindingMode::Dwarf
            }
        };

        let cg_mode = match cg_mode {
            UnwindingMode::FramePointer => "fp",
            UnwindingMode::Dwarf => "dwarf",
        };
        debug!("Using call graph mode: {cg_mode:?}");

        let quiet_flag = {
            let log_level = std::env::var("CODSPEED_LOG")
                .ok()
                .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
                .unwrap_or(log::LevelFilter::Info);

            if log_level < log::LevelFilter::Debug {
                "--quiet"
            } else {
                ""
            }
        };

        cmd.args([
            "sh",
            "-c",
            &format!(
                "perf record {quiet_flag} --user-callchains --freq=999 --switch-output --control=fifo:{},{} --delay=-1 -g --call-graph={cg_mode} --output={} -- {bench_cmd}",
                perf_fifo.ctl_fifo_path.to_string_lossy(),
                perf_fifo.ack_fifo_path.to_string_lossy(),
                perf_file.path().to_string_lossy(),
            ),
        ]);
        debug!("cmd: {cmd:?}");

        let on_process_started = async |_| -> anyhow::Result<()> {
            let data = Self::handle_fifo(runner_fifo, perf_fifo).await?;
            let _ = self.benchmark_data.set(data);

            Ok(())
        };

        {
            let _guard = EnvGuard::setup();
            run_command_with_log_pipe_and_callback(cmd, on_process_started).await
        }
    }

    pub async fn save_files_to(&self, profile_folder: &PathBuf) -> anyhow::Result<()> {
        let start = std::time::Instant::now();

        // We ran perf with sudo, so we have to change the ownership of the perf data files
        run_with_sudo(&[
            "chown",
            "-R",
            &format!(
                "{}:{}",
                nix::unistd::Uid::current(),
                nix::unistd::Gid::current()
            ),
            self.perf_dir.path().to_str().unwrap(),
        ])?;

        // Copy the perf data files to the profile folder
        let copy_tasks = std::fs::read_dir(&self.perf_dir)?
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
            .map(|src_path| {
                let profile_folder = profile_folder.clone();
                tokio::task::spawn(async move {
                    let pid = helpers::find_pid(&src_path)?;

                    let dst_file_name = format!(
                        "{}_{}.perf",
                        pid,
                        src_path.file_name().unwrap_or_default().to_string_lossy(),
                    );
                    let dst_path = profile_folder.join(dst_file_name);
                    tokio::fs::copy(src_path, dst_path).await?;

                    Ok::<_, anyhow::Error>(pid)
                })
            })
            .collect::<FuturesUnordered<_>>();

        let bench_data = self
            .benchmark_data
            .get()
            .expect("Benchmark order is not available");
        assert_eq!(
            copy_tasks.len(),
            bench_data.bench_count(),
            "Benchmark count mismatch"
        );

        // Harvest the perf maps generated by python. This will copy the perf
        // maps from /tmp to the profile folder. We have to write our own perf
        // maps to these files AFTERWARDS, otherwise it'll be overwritten!
        let perf_map_pids = futures::future::try_join_all(copy_tasks)
            .await?
            .into_iter()
            .filter_map(Result::ok)
            .collect::<HashSet<_>>();
        harvest_perf_maps_for_pids(profile_folder, &perf_map_pids).await?;

        // Append perf maps, unwind info and other metadata
        bench_data.save_to(profile_folder).unwrap();

        let elapsed = start.elapsed();
        debug!("Perf teardown took: {elapsed:?}");
        Ok(())
    }

    async fn handle_fifo(
        mut runner_fifo: RunnerFifo,
        mut perf_fifo: PerfFifo,
    ) -> anyhow::Result<BenchmarkData> {
        let mut bench_order_by_pid = HashMap::<u32, Vec<String>>::new();
        let mut symbols_by_pid = HashMap::<u32, ProcessSymbols>::new();
        let mut unwind_data_by_pid = HashMap::<u32, Vec<UnwindData>>::new();
        let mut integration = None;

        let perf_pid = OnceCell::new();
        loop {
            let perf_ping = tokio::time::timeout(Duration::from_secs(1), perf_fifo.ping()).await;
            if let Ok(Err(_)) | Err(_) = perf_ping {
                break;
            }

            let result = tokio::time::timeout(Duration::from_secs(5), runner_fifo.recv_cmd()).await;
            let Ok(Ok(cmd)) = result else {
                continue;
            };
            debug!("Received command: {cmd:?}");

            match cmd {
                FifoCommand::CurrentBenchmark { pid, uri } => {
                    bench_order_by_pid.entry(pid).or_default().push(uri);

                    #[cfg(target_os = "linux")]
                    if !symbols_by_pid.contains_key(&pid) && !unwind_data_by_pid.contains_key(&pid)
                    {
                        use procfs::process::MMPermissions;

                        let bench_proc = procfs::process::Process::new(pid as _)
                            .expect("Failed to find benchmark process");
                        let exe_maps = bench_proc.maps().expect("Failed to read /proc/{pid}/maps");

                        for map in &exe_maps {
                            let page_offset = map.offset;
                            let (base_addr, end_addr) = map.address;
                            let path = match &map.pathname {
                                procfs::process::MMapPath::Path(path) => Some(path.clone()),
                                _ => None,
                            };

                            if let Some(path) = &path {
                                symbols_by_pid
                                    .entry(pid)
                                    .or_insert(ProcessSymbols::new(pid))
                                    .add_mapping(pid, path, base_addr, end_addr);
                                debug!("Added mapping for module {path:?}");

                                if map.perms.contains(MMPermissions::EXECUTE) {
                                    match UnwindData::new(
                                        path.to_string_lossy().as_bytes(),
                                        page_offset,
                                        base_addr,
                                        end_addr - base_addr,
                                        None,
                                    ) {
                                        Ok(unwind_data) => {
                                            unwind_data_by_pid
                                                .entry(pid)
                                                .or_default()
                                                .push(unwind_data);
                                            debug!(
                                                "Added unwind data for {path:?} ({base_addr:x} - {end_addr:x})"
                                            );
                                        }
                                        Err(error) => {
                                            debug!(
                                                "Failed to create unwind data for module {}: {}",
                                                path.display(),
                                                error
                                            );
                                        }
                                    }
                                }
                            } else if map.perms.contains(MMPermissions::EXECUTE) {
                                debug!(
                                    "Found executable mapping without path: {base_addr:x} - {end_addr:x}"
                                );
                            }
                        }
                    }

                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::StartBenchmark => {
                    let perf_pid = perf_pid.get_or_init(|| {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg("pidof -s perf")
                            .output()
                            .expect("Failed to run pidof command");

                        String::from_utf8_lossy(&output.stdout)
                            .trim()
                            .parse::<u32>()
                            .expect("Failed to parse perf pid")
                    });

                    // Split the perf.data file
                    run_with_sudo(&["kill", "-USR2", &perf_pid.to_string()])?;

                    perf_fifo.start_events().await?;
                    runner_fifo.send_cmd(FifoCommand::Ack).await?;
                }
                FifoCommand::StopBenchmark => {
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
                FifoCommand::Ack => unreachable!(),
                FifoCommand::Err => unreachable!(),
            }
        }

        Ok(BenchmarkData {
            integration: integration.context("Couldn't find integration metadata")?,
            bench_order_by_pid,
            symbols_by_pid,
            unwind_data_by_pid,
        })
    }
}

pub struct BenchmarkData {
    /// Name and version of the integration
    integration: (String, String),

    bench_order_by_pid: HashMap<u32, Vec<String>>,
    symbols_by_pid: HashMap<u32, ProcessSymbols>,
    unwind_data_by_pid: HashMap<u32, Vec<UnwindData>>,
}

impl BenchmarkData {
    pub fn save_to<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        for proc_sym in self.symbols_by_pid.values() {
            proc_sym.save_to(&path).unwrap();
        }

        for (pid, modules) in &self.unwind_data_by_pid {
            for module in modules {
                module.save_to(&path, *pid).unwrap();
            }
        }

        let metadata = PerfMetadata {
            integration: self.integration.clone(),
            bench_order_by_pid: self.bench_order_by_pid.clone(),
            ignored_modules: {
                let mut to_ignore = vec![];

                // Check if any of the ignored modules has been loaded in the process
                for ignore_path in get_objects_path_to_ignore() {
                    for proc in self.symbols_by_pid.values() {
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

                to_ignore
            },
        };
        metadata.save_to(&path).unwrap();

        Ok(())
    }

    pub fn bench_count(&self) -> usize {
        self.bench_order_by_pid.values().map(|v| v.len()).sum()
    }
}
#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use std::{
        io::{Read, Write},
        os::unix::fs::PermissionsExt,
    };

    #[test]
    fn test_env_guard_no_crash() {
        fn create_run_script(content: &str) -> anyhow::Result<NamedTempFile> {
            let rwx = std::fs::Permissions::from_mode(0o777);
            let mut script_file = tempfile::Builder::new()
                .suffix(".sh")
                .permissions(rwx)
                .keep(true)
                .tempfile()?;
            script_file.write_all(content.as_bytes())?;

            Ok(script_file)
        }

        let mut tmp_dst = tempfile::NamedTempFile::new().unwrap();

        let pre_script = create_run_script(&format!(
            "#!/usr/bin/env bash\necho \"pre\" >> {}",
            tmp_dst.path().display()
        ))
        .unwrap();
        let post_script = create_run_script(&format!(
            "#!/usr/bin/env bash\necho \"post\" >> {}",
            tmp_dst.path().display()
        ))
        .unwrap();

        {
            let _guard = EnvGuard::setup_with_scripts(pre_script.path(), post_script.path());
        }

        let mut result = String::new();
        tmp_dst.read_to_string(&mut result).unwrap();
        assert_eq!(result, "pre\npost\n");
    }
}
