use console::style;
use tokio::time::{Instant, sleep};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalExecReportResponse, FetchLocalExecReportVars, RunStatus,
};
use crate::exec::DEFAULT_REPOSITORY_NAME;
use crate::prelude::*;
use crate::run::helpers::poll_results::{
    POLLING_INTERVAL, RUN_PROCESSING_MAX_DURATION, build_benchmark_table, retry_on_timeout,
};

#[allow(clippy::borrowed_box)]
pub async fn poll_results(api_client: &CodSpeedAPIClient, run_id: String) -> Result<()> {
    let start = Instant::now();
    let fetch_local_exec_report_vars = FetchLocalExecReportVars {
        // TODO: Set this dynamically based on the upload endpoint return value
        name: DEFAULT_REPOSITORY_NAME.to_owned(),
        run_id: run_id.clone(),
    };

    start_group!("Fetching the results");
    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        let fetch_result = retry_on_timeout(|| async {
            api_client
                .fetch_local_exec_report(fetch_local_exec_report_vars.clone())
                .await
        })
        .await?;

        match fetch_result {
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
