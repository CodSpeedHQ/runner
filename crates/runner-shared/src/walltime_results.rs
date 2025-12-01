// WARN: Keep in sync with codspeed-rust

use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use statrs::statistics::{Data, Distribution, Max, Min, OrderStatistics};
use std::path::Path;

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

impl WalltimeResults {
    pub fn from_benchmarks(benchmarks: Vec<WalltimeBenchmark>) -> Result<Self> {
        Ok(WalltimeResults {
            instrument: Instrument {
                type_: "walltime".to_string(),
            },
            creator: Creator {
                name: "codspeed-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                pid: std::process::id(),
            },
            benchmarks,
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, profile_folder: P) -> Result<()> {
        let results_path = {
            let results_dir = profile_folder.as_ref().join("results");
            std::fs::create_dir_all(&results_dir).with_context(|| {
                format!(
                    "Failed to create results directory: {}",
                    results_dir.display()
                )
            })?;

            results_dir.join(format!("{}.json", self.creator.pid))
        };

        let file = std::fs::File::create(&results_path)
            .with_context(|| format!("Failed to create file: {}", results_path.display()))?;
        serde_json::to_writer_pretty(file, &self)
            .with_context(|| format!("Failed to write JSON to file: {}", results_path.display()))?;
        Ok(())
    }
}

impl WalltimeBenchmark {
    pub fn from_runtime_data(
        name: String,
        uri: String,
        iters_per_round: Vec<u128>,
        times_per_round_ns: Vec<u128>,
        max_time_ns: Option<u128>,
    ) -> Self {
        const IQR_OUTLIER_FACTOR: f64 = 1.5;
        const STDEV_OUTLIER_FACTOR: f64 = 3.0;

        let total_time = times_per_round_ns.iter().sum::<u128>() as f64 / 1_000_000_000.0;
        let time_per_iteration_per_round_ns: Vec<_> = times_per_round_ns
            .into_iter()
            .zip(&iters_per_round)
            .map(|(time_per_round, iter_per_round)| time_per_round / iter_per_round)
            .map(|t| t as f64)
            .collect::<Vec<f64>>();

        let mut data = Data::new(time_per_iteration_per_round_ns);
        let rounds = data.len() as u64;

        let mean_ns = data.mean().unwrap();

        let stdev_ns = if data.len() < 2 {
            // std_dev() returns f64::NAN if data has less than two entries, so we have to
            // manually handle this case.
            0.0
        } else {
            data.std_dev().unwrap()
        };

        let q1_ns = data.quantile(0.25);
        let median_ns = data.median();
        let q3_ns = data.quantile(0.75);

        let iqr_ns = q3_ns - q1_ns;
        let iqr_outlier_rounds = data
            .iter()
            .filter(|&&t| {
                t < q1_ns - IQR_OUTLIER_FACTOR * iqr_ns || t > q3_ns + IQR_OUTLIER_FACTOR * iqr_ns
            })
            .count() as u64;

        let stdev_outlier_rounds = data
            .iter()
            .filter(|&&t| {
                t < mean_ns - STDEV_OUTLIER_FACTOR * stdev_ns
                    || t > mean_ns + STDEV_OUTLIER_FACTOR * stdev_ns
            })
            .count() as u64;

        let min_ns = data.min();
        let max_ns = data.max();

        // TODO(COD-1056): We currently only support single iteration count per round
        let iter_per_round =
            (iters_per_round.iter().sum::<u128>() / iters_per_round.len() as u128) as u64;
        let warmup_iters = 0; // FIXME: add warmup detection

        let stats = BenchmarkStats {
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
            warmup_iters,
        };

        WalltimeBenchmark {
            metadata: BenchmarkMetadata { name, uri },
            config: BenchmarkConfig {
                max_time_ns: max_time_ns.map(|t| t as f64),
                ..Default::default()
            },
            stats,
        }
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
