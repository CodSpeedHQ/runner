use crate::run::runner::helpers::setup::run_with_sudo;
use codspeed::fifo::FifoIpc;
use std::io::{Read, Write};

pub const PERF_CTL_FIFO: &str = "/tmp/codspeed_perf.ctl.fifo";
pub const PERF_CTL_ACK_FIFO: &str = "/tmp/codspeed_perf.ack.fifo";

pub struct PerfFifo {
    ctl_fifo: FifoIpc,
    ack_fifo: FifoIpc,
}

impl PerfFifo {
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

    pub fn new() -> Self {
        // Note: The writer can't be opened before there's a reader. We can
        // create our own reader first to be able to open the writer.
        let ctl_fifo = FifoIpc::create(PERF_CTL_FIFO)
            .unwrap()
            .with_reader()
            .unwrap()
            .with_writer()
            .unwrap();
        let ack_fifo = FifoIpc::create(PERF_CTL_ACK_FIFO)
            .unwrap()
            .with_reader()
            .unwrap();

        Self { ctl_fifo, ack_fifo }
    }

    pub fn start_events(&mut self) {
        self.ctl_fifo.write_all(b"enable\n").unwrap();
        self.wait_for_ack();
    }

    pub fn stop_events(&mut self) {
        self.ctl_fifo.write_all(b"disable\n").unwrap();
        self.wait_for_ack();
    }
}

pub fn setup_environment() {
    let sysctl_read = |name: &str| -> Option<u64> {
        let output = std::process::Command::new("sysctl")
            .arg(name)
            .output()
            .unwrap();
        let output = String::from_utf8(output.stdout).unwrap();
        output
            .split(" = ")
            .last()
            .unwrap()
            .trim()
            .parse::<u64>()
            .ok()
    };

    // Allow access to kernel symbols
    let kptr_restrict = sysctl_read("kernel.kptr_restrict");
    if kptr_restrict != Some(0) {
        run_with_sudo(&["sysctl", "-w", "kernel.kptr_restrict=0"]).unwrap();
    }

    // Allow non-root profiling
    let perf_event_paranoid = sysctl_read("kernel.perf_event_paranoid");
    if perf_event_paranoid != Some(1) {
        run_with_sudo(&["sysctl", "-w", "kernel.perf_event_paranoid=1"]).unwrap();
    }
}
