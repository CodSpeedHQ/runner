use crate::run::helpers;
use std::time::Duration;
use tabled::settings::Style;
use tabled::{Table, Tabled};

pub const RUN_PROCESSING_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes
pub const POLLING_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Tabled)]
struct BenchmarkRow {
    #[tabled(rename = "Benchmark")]
    name: String,
    #[tabled(rename = "Time")]
    time: String,
}

pub fn build_benchmark_table(
    results: &[crate::api_client::FetchLocalRunBenchmarkResult],
) -> String {
    let table_rows: Vec<BenchmarkRow> = results
        .iter()
        .map(|result| BenchmarkRow {
            name: result.benchmark.name.clone(),
            time: helpers::format_duration(result.time, Some(2)),
        })
        .collect();

    Table::new(&table_rows).with(Style::modern()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::{FetchLocalRunBenchmark, FetchLocalRunBenchmarkResult};

    #[test]
    fn test_benchmark_table_formatting() {
        let results = vec![
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_fast".to_string(),
                },
                time: 0.001234, // 1.23 ms
            },
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_slow".to_string(),
                },
                time: 1.5678, // 1.57 s
            },
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_medium".to_string(),
                },
                time: 0.000567, // 567 µs
            },
        ];

        let table = build_benchmark_table(&results);

        insta::assert_snapshot!(table, @r###"
        ┌──────────────────┬───────────┐
        │ Benchmark        │ Time      │
        ├──────────────────┼───────────┤
        │ benchmark_fast   │ 1.23 ms   │
        ├──────────────────┼───────────┤
        │ benchmark_slow   │ 1.57 s    │
        ├──────────────────┼───────────┤
        │ benchmark_medium │ 567.00 µs │
        └──────────────────┴───────────┘
        "###);
    }
}
