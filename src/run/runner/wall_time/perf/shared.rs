// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH BELOW !!
// !!!!!!!!!!!!!!!!!!!!!!!!
// Has to be in sync with `runner`.
//
pub const RUNNER_CTL_FIFO: &str = "/tmp/runner.ctl.fifo";
pub const RUNNER_ACK_FIFO: &str = "/tmp/runner.ack.fifo";

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
