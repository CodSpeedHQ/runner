use std::path::{Path, PathBuf};

// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH BELOW !!
// !!!!!!!!!!!!!!!!!!!!!!!!
// Has to be in sync with `codspeed-rust/codspeed`.
//
const RUNNER_CTL_FIFO_NAME: &str = "runner.ctl.fifo";
const RUNNER_ACK_FIFO_NAME: &str = "runner.ack.fifo";

pub fn set_runner_fifo_dir<P: AsRef<Path>>(path: P) {
    std::env::set_var("CODSPEED_FIFO_DIR", path.as_ref());
}

pub fn runner_fifo_dir() -> PathBuf {
    let raw_path =
        std::env::var("CODSPEED_FIFO_DIR").expect("CODSPEED_FIFO_DIR environment variable not set");
    PathBuf::from(raw_path)
}

pub fn runner_ctl_fifo_path() -> PathBuf {
    runner_fifo_dir().join(RUNNER_CTL_FIFO_NAME)
}

pub fn runner_ack_fifo_path() -> PathBuf {
    runner_fifo_dir().join(RUNNER_ACK_FIFO_NAME)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub enum Command {
    CurrentBenchmark { pid: u32, uri: String },
    StartBenchmark,
    StopBenchmark,
    Ack,
}
//
// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH ABOVE !!
// !!!!!!!!!!!!!!!!!!!!!!!!
