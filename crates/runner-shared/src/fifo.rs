//! WARNING: Has to be in sync with `instrument-hooks`.

pub const RUNNER_CTL_FIFO: &str = "/tmp/runner.ctl.fifo";
pub const RUNNER_ACK_FIFO: &str = "/tmp/runner.ack.fifo";

pub const CURRENT_PROTOCOL_VERSION: u64 = 2;

/// The different markers that can be set in the perf.data.
///
/// `SampleStart/End`: Marks the start and end of a sampling period. This is used to differentiate between benchmarks.
/// `BenchmarkStart/End`: Marks the start and end of a benchmark. This is used to measure the duration of a benchmark, without the benchmark harness code.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone,
)]
pub enum MarkerType {
    SampleStart(u64),
    SampleEnd(u64),
    BenchmarkStart(u64),
    BenchmarkEnd(u64),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMode {
    Perf,
    Simulation,
    Analysis,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub enum Command {
    CurrentBenchmark {
        pid: i32,
        uri: String,
    },
    StartBenchmark,
    StopBenchmark,
    Ack,
    #[deprecated(note = "Use `GetIntegrationMode` instead")]
    PingPerf,
    SetIntegration {
        name: String,
        version: String,
    },
    Err,
    AddMarker {
        pid: i32,
        marker: MarkerType,
    },
    SetVersion(u64),
    GetIntegrationMode,
    IntegrationModeResponse(IntegrationMode),
}
