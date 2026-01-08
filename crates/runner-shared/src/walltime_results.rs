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

impl WalltimeBenchmark {
    pub fn from_runtime_data(
        name: String,
        uri: String,
        iters_per_round: Vec<u64>,
        times_per_round_ns: Vec<u128>,
        _max_time_ns: Option<u128>,
    ) -> Self {
        // Calculate statistics
        let mut times_sorted: Vec<f64> = times_per_round_ns.iter().map(|&t| t as f64).collect();
        times_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let rounds = times_sorted.len() as u64;
        let total_time: f64 = times_sorted.iter().sum();
        let mean_ns = if rounds > 0 {
            total_time / rounds as f64
        } else {
            0.0
        };

        let min_ns = times_sorted.first().copied().unwrap_or(0.0);
        let max_ns = times_sorted.last().copied().unwrap_or(0.0);

        // Calculate percentiles
        let median_ns = if rounds > 0 {
            let mid = rounds as usize / 2;
            if rounds % 2 == 0 {
                (times_sorted[mid - 1] + times_sorted[mid]) / 2.0
            } else {
                times_sorted[mid]
            }
        } else {
            0.0
        };

        let q1_ns = if rounds > 0 {
            let q1_idx = (rounds as usize / 4).max(0);
            times_sorted[q1_idx]
        } else {
            0.0
        };

        let q3_ns = if rounds > 0 {
            let q3_idx = (3 * rounds as usize / 4).min(times_sorted.len() - 1);
            times_sorted[q3_idx]
        } else {
            0.0
        };

        // Calculate standard deviation
        let stdev_ns = if rounds > 1 {
            let variance: f64 = times_sorted
                .iter()
                .map(|&t| {
                    let diff = t - mean_ns;
                    diff * diff
                })
                .sum::<f64>()
                / (rounds - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        // Calculate outliers (simplified - using IQR method)
        let iqr = q3_ns - q1_ns;
        let lower_bound = q1_ns - 1.5 * iqr;
        let upper_bound = q3_ns + 1.5 * iqr;
        let iqr_outlier_rounds = times_sorted
            .iter()
            .filter(|&&t| t < lower_bound || t > upper_bound)
            .count() as u64;

        // Standard deviation outliers (2 sigma)
        let stdev_outlier_rounds = times_sorted
            .iter()
            .filter(|&&t| (t - mean_ns).abs() > 2.0 * stdev_ns)
            .count() as u64;

        let iter_per_round = if !iters_per_round.is_empty() {
            iters_per_round[0]
        } else {
            1
        };

        WalltimeBenchmark {
            metadata: BenchmarkMetadata { name, uri },
            config: BenchmarkConfig::default(),
            stats: BenchmarkStats {
                min_ns,
                max_ns,
                mean_ns,
                stdev_ns,
                q1_ns,
                median_ns,
                q3_ns,
                rounds,
                total_time,
                iqr_outlier_rounds,
                stdev_outlier_rounds,
                iter_per_round,
                warmup_iters: 0,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Instrument {
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Creator {
    pub name: String,
    pub version: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalltimeResults {
    pub creator: Creator,
    pub instrument: Instrument,
    pub benchmarks: Vec<WalltimeBenchmark>,
}

impl WalltimeResults {
    pub fn from_benchmarks(benchmarks: Vec<WalltimeBenchmark>) -> anyhow::Result<Self> {
        Ok(WalltimeResults {
            instrument: Instrument {
                type_: "walltime".to_string(),
            },
            creator: Creator {
                // TODO: Stop impersonating codspeed-rust 🥸
                name: "codspeed-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                pid: std::process::id(),
            },
            benchmarks,
        })
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, profile_folder: P) -> anyhow::Result<()> {
        let results_path = {
            let results_dir = profile_folder.as_ref().join("results");
            std::fs::create_dir_all(&results_dir).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create results directory: {}: {}",
                    results_dir.display(),
                    e
                )
            })?;

            results_dir.join(format!("{}.json", self.creator.pid))
        };

        let file = std::fs::File::create(&results_path).map_err(|e| {
            anyhow::anyhow!("Failed to create file: {}: {}", results_path.display(), e)
        })?;
        serde_json::to_writer_pretty(file, &self).map_err(|e| {
            anyhow::anyhow!(
                "Failed to write JSON to file: {}: {}",
                results_path.display(),
                e
            )
        })?;
        Ok(())
    }
}
