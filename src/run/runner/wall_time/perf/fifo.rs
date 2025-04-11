use super::perf_map::ProcessSymbols;
use super::{
    runner_ack_fifo_path, runner_ctl_fifo_path, set_runner_fifo_dir, BenchmarkData,
    Command as FifoCommand,
};
use crate::prelude::*;
use crate::run::runner::wall_time::perf::perf_map::ModuleSymbols;
use crate::run::runner::wall_time::perf::unwind_data::UnwindData;
use procfs::process::MMPermissions;
use std::collections::HashMap;
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::pipe::OpenOptions as TokioPipeOpenOptions;
use tokio::net::unix::pipe::Receiver as TokioPipeReader;
use tokio::net::unix::pipe::Sender as TokioPipeSender;

fn create_fifo<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<()> {
    // Remove the previous FIFO (if it exists)
    let _ = nix::unistd::unlink(path.as_ref());

    // Create the FIFO with RWX permissions for the owner
    nix::unistd::mkfifo(path.as_ref(), nix::sys::stat::Mode::S_IRWXU)?;

    Ok(())
}

pub struct RunnerFifo {
    ack_fifo: TokioPipeSender,
    ctl_fifo: TokioPipeReader,
}

impl RunnerFifo {
    pub fn new() -> anyhow::Result<Self> {
        set_runner_fifo_dir(tempfile::tempdir()?.into_path());

        create_fifo(runner_ctl_fifo_path()?)?;
        create_fifo(runner_ack_fifo_path()?)?;

        let ack_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_sender(runner_ack_fifo_path()?)?;
        let ctl_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_receiver(runner_ctl_fifo_path()?)?;

        Ok(Self { ctl_fifo, ack_fifo })
    }

    pub async fn recv_cmd(&mut self) -> anyhow::Result<FifoCommand> {
        let mut len_buffer = [0u8; 4];
        self.ctl_fifo.read_exact(&mut len_buffer).await?;
        let message_len = u32::from_le_bytes(len_buffer) as usize;

        let mut buffer = vec![0u8; message_len];
        loop {
            if self.ctl_fifo.read_exact(&mut buffer).await.is_ok() {
                break;
            }
        }

        let decoded = bincode::deserialize(&buffer)?;
        Ok(decoded)
    }

    pub async fn send_cmd(&mut self, cmd: FifoCommand) -> anyhow::Result<()> {
        let encoded = bincode::serialize(&cmd)?;

        self.ack_fifo
            .write_all(&(encoded.len() as u32).to_le_bytes())
            .await?;
        self.ack_fifo.write_all(&encoded).await?;
        Ok(())
    }
}

pub struct PerfFifo {
    ctl_fifo: TokioPipeSender,
    ack_fifo: TokioPipeReader,

    pub(crate) ctl_fifo_path: PathBuf,
    pub(crate) ack_fifo_path: PathBuf,
}

impl PerfFifo {
    pub fn new() -> anyhow::Result<Self> {
        let fifo_dir = tempfile::tempdir()?.into_path();

        let ctl_fifo_path = fifo_dir.join("codspeed_perf.ctl.fifo");
        let ack_fifo_path = fifo_dir.join("codspeed_perf.ack.fifo");

        create_fifo(&ctl_fifo_path)?;
        create_fifo(&ack_fifo_path)?;

        let ack_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_receiver(&ack_fifo_path)?;
        let ctl_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_sender(&ctl_fifo_path)?;

        Ok(Self {
            ctl_fifo,
            ack_fifo,
            ctl_fifo_path,
            ack_fifo_path,
        })
    }

    pub async fn start_events(&mut self) -> anyhow::Result<()> {
        self.ctl_fifo.write_all(b"enable\n").await?;
        self.wait_for_ack().await;

        Ok(())
    }

    pub async fn stop_events(&mut self) -> anyhow::Result<()> {
        self.ctl_fifo.write_all(b"disable\n").await?;
        self.wait_for_ack().await;

        Ok(())
    }

    pub async fn ping(&mut self) -> anyhow::Result<()> {
        self.ctl_fifo.write_all(b"ping\n").await?;
        self.wait_for_ack().await;

        Ok(())
    }

    async fn wait_for_ack(&mut self) {
        const ACK: &[u8] = b"ack\n\x00";

        loop {
            let mut buf: [u8; ACK.len()] = [0; ACK.len()];
            if self.ack_fifo.read_exact(&mut buf).await.is_err() {
                continue;
            }

            if buf == ACK {
                break;
            }
        }
    }
}

pub async fn handle_fifo(
    perf_pid: u32,
    mut runner_fifo: RunnerFifo,
    mut perf_fifo: PerfFifo,
) -> anyhow::Result<BenchmarkData> {
    let mut bench_order_by_pid = HashMap::<u32, Vec<String>>::new();
    let mut symbols_by_pid = HashMap::<u32, ProcessSymbols>::new();
    let mut unwind_data_by_pid = HashMap::<u32, Vec<UnwindData>>::new();

    loop {
        let perf_ping = tokio::time::timeout(Duration::from_secs(1), perf_fifo.ping()).await;
        if let Ok(Err(_)) | Err(_) = perf_ping {
            break;
        }

        let result = tokio::time::timeout(Duration::from_secs(1), runner_fifo.recv_cmd()).await;
        let Ok(Ok(cmd)) = result else {
            continue;
        };
        debug!("Received command: {:?}", cmd);

        match cmd {
            FifoCommand::CurrentBenchmark { pid, uri } => {
                bench_order_by_pid.entry(pid).or_default().push(uri);

                if !symbols_by_pid.contains_key(&pid) && !unwind_data_by_pid.contains_key(&pid) {
                    let bench_proc = procfs::process::Process::new(pid as _)
                        .expect("Failed to find benchmark process");
                    let exe_path = bench_proc.exe().expect("Failed to read /proc/{pid}/exe");
                    let exe_maps = bench_proc.maps().expect("Failed to read /proc/{pid}/maps");

                    for map in &exe_maps {
                        let page_offset = map.offset;
                        let (base_addr, end_addr) = map.address;
                        let path = match &map.pathname {
                            procfs::process::MMapPath::Path(path) => Some(path.clone()),
                            _ => None,
                        };

                        if let Some(path) = path {
                            if let Some(symbols) = ModuleSymbols::new(pid, &path, base_addr) {
                                symbols_by_pid
                                    .entry(pid)
                                    .or_insert(ProcessSymbols::new(pid))
                                    .add_module_symbols(symbols);
                            }
                        }

                        if map.perms.contains(MMPermissions::EXECUTE) {
                            if let Ok(unwind_data) = UnwindData::new(
                                exe_path.to_string_lossy().as_bytes(),
                                page_offset,
                                base_addr,
                                end_addr - base_addr,
                                None,
                            ) {
                                unwind_data_by_pid.entry(pid).or_default().push(unwind_data);
                            }
                        }
                    }
                }

                runner_fifo.send_cmd(FifoCommand::Ack).await?;
            }
            FifoCommand::StartBenchmark => {
                unsafe { libc::kill(perf_pid as i32, libc::SIGUSR2) };
                perf_fifo.start_events().await?;
                runner_fifo.send_cmd(FifoCommand::Ack).await?;
            }
            FifoCommand::StopBenchmark => {
                perf_fifo.stop_events().await?;
                runner_fifo.send_cmd(FifoCommand::Ack).await?;
            }
            FifoCommand::Ack => unreachable!(),
        }
    }

    Ok(BenchmarkData {
        bench_order_by_pid,
        symbols_by_pid,
        unwind_data_by_pid,
    })
}
