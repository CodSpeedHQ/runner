use crate::api_client::FetchLocalRunBenchmarkResult;
use crate::executor::ExecutorName;
use crate::run::helpers;
use std::time::Duration;
use tabled::settings::Style;
use tabled::{Table, Tabled};

pub const RUN_PROCESSING_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes
pub const POLLING_INTERVAL: Duration = Duration::from_secs(1);

fn format_measurement(value: f64, executor: &ExecutorName) -> String {
    match executor {
        ExecutorName::Memory => helpers::format_memory(value, Some(1)),
        ExecutorName::Valgrind | ExecutorName::WallTime => helpers::format_duration(value, Some(2)),
    }
}

#[derive(Tabled)]
struct BenchmarkRow {
    #[tabled(rename = "Benchmark")]
    name: String,
    #[tabled(rename = "Measurement")]
    measurement: String,
}

pub fn build_benchmark_table(results: &[FetchLocalRunBenchmarkResult]) -> String {
    let table_rows: Vec<BenchmarkRow> = results
        .iter()
        .map(|result| BenchmarkRow {
            name: result.benchmark.name.clone(),
            measurement: format_measurement(result.value, &result.benchmark.executor),
        })
        .collect();

    Table::new(&table_rows).with(Style::modern()).to_string()
}

pub fn build_detailed_summary(result: &FetchLocalRunBenchmarkResult) -> String {
    format!(
        "{}: {}",
        result.benchmark.name,
        format_measurement(result.value, &result.benchmark.executor)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api_client::FetchLocalRunBenchmark, executor::ExecutorName};

    #[test]
    fn test_benchmark_table_formatting() {
        let results = vec![
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_fast".to_string(),
                    executor: ExecutorName::Valgrind,
                },
                value: 0.001234, // 1.23 ms
            },
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_slow".to_string(),
                    executor: ExecutorName::WallTime,
                },
                value: 1.5678, // 1.57 s
            },
            FetchLocalRunBenchmarkResult {
                benchmark: FetchLocalRunBenchmark {
                    name: "benchmark_memory".to_string(),
                    executor: ExecutorName::Memory,
                },
                value: 2097152.0, // 2 MB (2 * 1024^2)
            },
        ];

        let table = build_benchmark_table(&results);

        insta::assert_snapshot!(table, @r###"
        ┌──────────────────┬─────────────┐
        │ Benchmark        │ Measurement │
        ├──────────────────┼─────────────┤
        │ benchmark_fast   │ 1.23 ms     │
        ├──────────────────┼─────────────┤
        │ benchmark_slow   │ 1.57 s      │
        ├──────────────────┼─────────────┤
        │ benchmark_memory │ 2 MB        │
        └──────────────────┴─────────────┘
        "###);
    }

    #[test]
    fn test_detailed_summary_formatting() {
        let result = FetchLocalRunBenchmarkResult {
            benchmark: FetchLocalRunBenchmark {
                name: "benchmark_fast".to_string(),
                executor: ExecutorName::Valgrind,
            },
            value: 0.001234, // 1.23 ms
        };

        let summary = build_detailed_summary(&result);

        insta::assert_snapshot!(summary, @"benchmark_fast: 1.23 ms");
    }
}
