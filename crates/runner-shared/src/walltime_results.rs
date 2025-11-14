// WARN: Keep in sync with codspeed-rust

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkMetadata {
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchmarkStats {
    pub min_ns: f64,
    pub max_ns: f64,
    pub mean_ns: f64,
    pub stdev_ns: f64,

    pub q1_ns: f64,
    pub median_ns: f64,
    pub q3_ns: f64,

    pub rounds: u64,
    pub total_time: f64,
    pub iqr_outlier_rounds: u64,
    pub stdev_outlier_rounds: u64,
    pub iter_per_round: u64,
    pub warmup_iters: u64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct BenchmarkConfig {
    warmup_time_ns: Option<f64>,
    min_round_time_ns: Option<f64>,
    max_time_ns: Option<f64>,
    max_rounds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalltimeBenchmark {
    #[serde(flatten)]
    pub metadata: BenchmarkMetadata,

    config: BenchmarkConfig,
    pub stats: BenchmarkStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Instrument {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Creator {
    name: String,
    version: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalltimeResults {
    pub creator: Creator,
    pub instrument: Instrument,
    pub benchmarks: Vec<WalltimeBenchmark>,
}
