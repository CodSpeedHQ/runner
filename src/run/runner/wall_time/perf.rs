use crate::run::runner::helpers::setup::run_with_sudo;
use anyhow::Context;
use codspeed::fifo::FifoIpc;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

pub struct PerfFifo {
    ctl_fifo: FifoIpc,
    ack_fifo: FifoIpc,

    pub(crate) ctl_fifo_path: PathBuf,
    pub(crate) ack_fifo_path: PathBuf,
}

impl PerfFifo {
    pub fn new() -> anyhow::Result<Self> {
        let fifo_dir = tempfile::tempdir()?;

        let make_fifo_path = |name: &str| -> PathBuf {
            let mut path = PathBuf::from(fifo_dir.path());
            path.push(name);
            path
        };

        let ctl_fifo_path = make_fifo_path("codspeed_perf.ctl.fifo");
        let ack_fifo_path = make_fifo_path("codspeed_perf.ack.fifo");

        // Note: The writer can't be opened before there's a reader. We can
        // create our own reader first to be able to open the writer.
        let ctl_fifo = FifoIpc::create(&ctl_fifo_path)?
            .with_reader()?
            .with_writer()?;
        let ack_fifo = FifoIpc::create(&ack_fifo_path)?.with_reader()?;

        Ok(Self {
            ctl_fifo,
            ack_fifo,
            ctl_fifo_path,
            ack_fifo_path,
        })
    }

    pub fn start_events(&mut self) -> anyhow::Result<()> {
        self.ctl_fifo.write_all(b"enable\n").unwrap();
        self.wait_for_ack();

        Ok(())
    }

    pub fn stop_events(&mut self) -> anyhow::Result<()> {
        self.ctl_fifo.write_all(b"disable\n").unwrap();
        self.wait_for_ack();

        Ok(())
    }

    fn wait_for_ack(&mut self) {
        const ACK: &[u8] = b"ack\n\x00";

        loop {
            let mut buf: [u8; ACK.len()] = [0; ACK.len()];
            if self.ack_fifo.read_exact(&mut buf).is_err() {
                continue;
            }

            if buf == ACK {
                break;
            }
        }
    }
}

pub fn setup_environment() -> anyhow::Result<()> {
    let sysctl_read = |name: &str| -> anyhow::Result<u64> {
        let output = std::process::Command::new("sysctl").arg(name).output()?;
        let output = String::from_utf8(output.stdout)?;

        Ok(output
            .split(" = ")
            .last()
            .context("Couldn't find the value in sysctl output")?
            .trim()
            .parse::<u64>()?)
    };

    // Allow access to kernel symbols
    if sysctl_read("kernel.kptr_restrict")? != 0 {
        run_with_sudo(&["sysctl", "-w", "kernel.kptr_restrict=0"])?;
    }

    // Allow non-root profiling
    if sysctl_read("kernel.perf_event_paranoid")? != 1 {
        run_with_sudo(&["sysctl", "-w", "kernel.perf_event_paranoid=1"])?;
    }

    Ok(())
}
