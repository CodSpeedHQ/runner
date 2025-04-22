use super::{FifoCommand, RUNNER_ACK_FIFO, RUNNER_CTL_FIFO};
use std::path::PathBuf;
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
        create_fifo(RUNNER_CTL_FIFO)?;
        create_fifo(RUNNER_ACK_FIFO)?;

        let ack_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_sender(RUNNER_ACK_FIFO)?;
        let ctl_fifo = TokioPipeOpenOptions::new()
            .read_write(true)
            .open_receiver(RUNNER_CTL_FIFO)?;

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
