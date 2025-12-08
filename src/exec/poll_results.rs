use std::time::Duration;

use console::style;
use tabled::settings::Style;
use tabled::{Table, Tabled};
use tokio::time::{Instant, sleep};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalExecReportResponse, FetchLocalExecReportVars, RunStatus,
};
use crate::prelude::*;
use crate::run::helpers;

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
pub async fn poll_results(api_client: &CodSpeedAPIClient, run_id: String) -> Result<()> {
    let start = Instant::now();
    let fetch_local_exec_report_vars = FetchLocalExecReportVars {
        // TODO: Set this dynamically based on the upload endpoint return value
        name: "local-runs".to_owned(),
        run_id: run_id.clone(),
    };

    start_group!("Fetching the results");
    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        match api_client
            .fetch_local_exec_report(fetch_local_exec_report_vars.clone())
            .await?
        {
            FetchLocalExecReportResponse { run, .. }
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

    info!(
        "\nTo see the full report, visit: {}",
        style(response.run.url).blue().bold().underlined()
    );
    end_group!();

    if !response.run.results.is_empty() {
        start_group!("Benchmark results");

        let table = build_benchmark_table(&response.run.results);
        info!("\n{table}");

        end_group!();
    }

    Ok(())
}
