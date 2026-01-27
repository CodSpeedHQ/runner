use console::style;
use tokio::time::{Instant, sleep};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalRunReportResponse, FetchLocalRunReportVars, RunStatus,
};
use crate::cli::run::helpers::benchmark_display::{
    POLLING_INTERVAL, RUN_PROCESSING_MAX_DURATION, build_benchmark_table, build_detailed_summary,
};
use crate::cli::run::uploader::UploadResult;
use crate::prelude::*;

#[allow(clippy::borrowed_box)]
pub async fn poll_results(
    api_client: &CodSpeedAPIClient,
    upload_result: &UploadResult,
) -> Result<()> {
    let start = Instant::now();
    let fetch_local_run_report_vars = FetchLocalRunReportVars {
        owner: upload_result.owner.clone(),
        name: upload_result.repository.clone(),
        run_id: upload_result.run_id.clone(),
    };

    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        let fetch_result = api_client
            .fetch_local_run_report(fetch_local_run_report_vars.clone())
            .await?;

        match fetch_result {
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

    if !response.run.results.is_empty() {
        end_group!();
        start_group!("Benchmark results");

        if response.run.results.len() == 1 {
            let summary = build_detailed_summary(&response.run.results[0]);
            info!("\n{summary}");
        } else {
            let table = build_benchmark_table(&response.run.results);
            info!("\n{table}");
        }

        info!(
            "\nTo see the full report, visit: {}",
            style(response.run.url).blue().bold().underlined()
        );
    }

    Ok(())
}
