use std::time::Duration;

use console::style;
use tabled::settings::Style;
use tabled::{Table, Tabled};
use tokio::time::{Instant, sleep};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalRunReportResponse, FetchLocalRunReportVars, RunStatus,
};
use crate::prelude::*;
use crate::run::helpers;
use crate::run_environment::RunEnvironmentProvider;

const RUN_PROCESSING_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes
const POLLING_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Tabled)]
struct BenchmarkRow {
    #[tabled(rename = "Benchmark")]
    name: String,
    #[tabled(rename = "Time")]
    time: String,
}

fn build_benchmark_table(results: &[crate::api_client::FetchLocalRunBenchmarkResult]) -> String {
    let table_rows: Vec<BenchmarkRow> = results
        .iter()
        .map(|result| BenchmarkRow {
            name: result.benchmark.name.clone(),
            time: helpers::format_duration(result.time, Some(2)),
        })
        .collect();

    Table::new(&table_rows).with(Style::modern()).to_string()
}

#[allow(clippy::borrowed_box)]
pub async fn poll_results(
    api_client: &CodSpeedAPIClient,
    provider: &Box<dyn RunEnvironmentProvider>,
    run_id: String,
    output_json: bool,
) -> Result<()> {
    let start = Instant::now();
    let run_environment_metadata = provider.get_run_environment_metadata()?;
    let owner = run_environment_metadata.owner;
    let name = run_environment_metadata.repository;
    let fetch_local_run_report_vars = FetchLocalRunReportVars {
        owner: owner.clone(),
        name: name.clone(),
        run_id: run_id.clone(),
    };

    start_group!("Fetching the results");
    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        match api_client
            .fetch_local_run_report(fetch_local_run_report_vars.clone())
            .await?
        {
            FetchLocalRunReportResponse { run, .. }
                if run.status == RunStatus::Pending || run.status == RunStatus::Processing =>
            {
                sleep(POLLING_INTERVAL).await;
            }
            response_from_api => {
                response = response_from_api;
                break;
            }
        }
    }

    if response.run.status == RunStatus::Failure {
        bail!("Run failed to be processed, try again in a few minutes");
    }

    let report = response
        .run
        .head_reports
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No head report found in the run report"))?;

    if let Some(impact) = report.impact {
        let rounded_impact = (impact * 100.0).round();
        let impact_text = if impact > 0.0 {
            style(format!("+{rounded_impact}%")).green().bold()
        } else {
            style(format!("{rounded_impact}%")).red().bold()
        };

        info!(
            "Impact: {} (allowed regression: -{}%)",
            impact_text,
            (response.allowed_regression * 100.0).round()
        );
    } else {
        info!("No impact detected, reason: {}", report.conclusion);
    }

    info!(
        "\nTo see the full report, visit: {}",
        style(response.run.url).blue().bold().underlined()
    );

    if output_json {
        // TODO: Refactor `log_json` to avoid having to format the json manually
        // We could make use of structured logging for this https://docs.rs/log/latest/log/#structured-logging
        log_json!(format!(
            "{{\"event\": \"run_finished\", \"run_id\": \"{}\"}}",
            run_id
        ));
    }

    end_group!();

    if !response.run.results.is_empty() {
        start_group!("Benchmark results");

        let table = build_benchmark_table(&response.run.results);
        info!("\n{table}");

        if output_json {
            for result in response.run.results {
                log_json!(format!(
                    "{{\"event\": \"benchmark_ran\", \"name\": \"{}\", \"time\": \"{}\"}}",
                    result.benchmark.name, result.time,
                ));
            }
        }

        end_group!();
    }

    Ok(())
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
