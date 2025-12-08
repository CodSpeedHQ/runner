use console::style;
use tokio::time::{Instant, sleep};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalRunReportResponse, FetchLocalRunReportVars, RunStatus,
};
use crate::prelude::*;
use crate::run::helpers::poll_results::{
    POLLING_INTERVAL, RUN_PROCESSING_MAX_DURATION, build_benchmark_table, retry_on_timeout,
};
use crate::run_environment::RunEnvironmentMetadata;

#[allow(clippy::borrowed_box)]
pub async fn poll_results(
    api_client: &CodSpeedAPIClient,
    run_environment_metadata: &RunEnvironmentMetadata,
    run_id: String,
    output_json: bool,
) -> Result<()> {
    let start = Instant::now();
    let owner = run_environment_metadata.owner.as_str();
    let name = run_environment_metadata.repository.as_str();
    let fetch_local_run_report_vars = FetchLocalRunReportVars {
        owner: owner.to_owned(),
        name: name.to_owned(),
        run_id: run_id.to_owned(),
    };

    start_group!("Fetching the results");
    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        let fetch_result = retry_on_timeout(|| async {
            api_client
                .fetch_local_run_report(fetch_local_run_report_vars.clone())
                .await
        })
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
