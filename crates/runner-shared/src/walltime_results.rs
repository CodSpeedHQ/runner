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
    /// Total time in **seconds**
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
        iters_per_round: Vec<u128>,
        times_per_round_ns: Vec<u128>,
        _max_time_ns: Option<u128>,
    ) -> Self {
        // Calculate total time in ⚠️ seconds ⚠️
        let total_time_s = times_per_round_ns.iter().sum::<u128>() as f64 / 1_000_000_000.0;

        // Calculate statistics
        let times_per_iteration_per_round_ns: Vec<_> = times_per_round_ns
            .into_iter()
            .zip(&iters_per_round)
            .map(|(time_per_round, iter_per_round)| time_per_round / iter_per_round)
            .map(|t| t as f64)
            .collect::<Vec<f64>>();

        let rounds = times_per_iteration_per_round_ns.len();
        let mean_ns = if rounds > 0 {
            times_per_iteration_per_round_ns.iter().sum::<f64>() / rounds as f64
        } else {
            0.0
        };

        let min_ns = times_per_iteration_per_round_ns
            .first()
            .copied()
            .unwrap_or(0.0);
        let max_ns = times_per_iteration_per_round_ns
            .last()
            .copied()
            .unwrap_or(0.0);

        // Calculate percentiles
        let median_ns = if rounds > 0 {
            let mid = rounds / 2;
            if rounds % 2 == 0 {
                (times_per_iteration_per_round_ns[mid - 1] + times_per_iteration_per_round_ns[mid])
                    / 2.0
            } else {
                times_per_iteration_per_round_ns[mid]
            }
        } else {
            0.0
        };

        let q1_ns = if rounds > 0 {
            let q1_idx = (rounds / 4).max(0);
            times_per_iteration_per_round_ns[q1_idx]
        } else {
            0.0
        };

        let q3_ns = if rounds > 0 {
            let q3_idx = (3 * rounds / 4).min(times_per_iteration_per_round_ns.len() - 1);
            times_per_iteration_per_round_ns[q3_idx]
        } else {
            0.0
        };

        // Calculate standard deviation
        let stdev_ns = if rounds > 1 {
            let variance: f64 = times_per_iteration_per_round_ns
                .iter()
                .map(|&t| {
                    let diff = t - mean_ns;
                    diff * diff
                })
                .sum::<f64>()
                / rounds as f64;
            variance.sqrt()
        } else {
            0.0
        };

        // Calculate outliers (simplified - using IQR method)
        let iqr = q3_ns - q1_ns;
        let lower_bound = q1_ns - 1.5 * iqr;
        let upper_bound = q3_ns + 1.5 * iqr;
        let iqr_outlier_rounds = times_per_iteration_per_round_ns
            .iter()
            .filter(|&&t| t < lower_bound || t > upper_bound)
            .count() as u64;

        // Standard deviation outliers (2 sigma)
        let stdev_outlier_rounds = times_per_iteration_per_round_ns
            .iter()
            .filter(|&&t| (t - mean_ns).abs() > 2.0 * stdev_ns)
            .count() as u64;

        // TODO(COD-1056): We currently only support single iteration count per round
        let iter_per_round =
            (iters_per_round.iter().sum::<u128>() / iters_per_round.len() as u128) as u64;

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
                rounds: rounds as u64,
                total_time: total_time_s,
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

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &str = "benchmark";
    const URI: &str = "test::benchmark";

    #[test]
    fn test_parse_single_benchmark() {
        let benchmark = WalltimeBenchmark::from_runtime_data(
            NAME.to_string(),
            URI.to_string(),
            vec![1],
            vec![42],
            None,
        );
        assert_eq!(benchmark.stats.stdev_ns, 0.);
        assert_eq!(benchmark.stats.min_ns, 42.);
        assert_eq!(benchmark.stats.max_ns, 42.);
        assert_eq!(benchmark.stats.mean_ns, 42.);
    }

    #[test]
    fn test_parse_bench_with_variable_iterations() {
        let iters_per_round = vec![1, 2, 3, 4, 5, 6];
        let total_rounds = iters_per_round.iter().sum::<u128>() as f64;

        let benchmark = WalltimeBenchmark::from_runtime_data(
            NAME.to_string(),
            URI.to_string(),
            iters_per_round,
            vec![42, 42 * 2, 42 * 3, 42 * 4, 42 * 5, 42 * 6],
            None,
        );

        dbg!(vec![42, 42 * 2, 42 * 3, 42 * 4, 42 * 5, 42 * 6]);

        assert_eq!(benchmark.stats.stdev_ns, 0.);
        assert_eq!(benchmark.stats.min_ns, 42.);
        assert_eq!(benchmark.stats.max_ns, 42.);
        assert_eq!(benchmark.stats.mean_ns, 42.);
        assert_eq!(
            benchmark.stats.total_time,
            42. * total_rounds / 1_000_000_000.0
        );
    }
}
