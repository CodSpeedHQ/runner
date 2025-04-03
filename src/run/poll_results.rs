use std::time::Duration;

use console::style;
use tokio::time::{sleep, Instant};

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalRunReportResponse, FetchLocalRunReportVars, RunStatus,
};
use crate::prelude::*;

use super::run_environment::RunEnvironmentProvider;

const RUN_PROCESSING_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes
const POLLING_INTERVAL: Duration = Duration::from_secs(1);

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
            style(format!("+{}%", rounded_impact)).green().bold()
        } else {
            style(format!("{}%", rounded_impact)).red().bold()
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
        start_group!("Benchmarks");
        for result in response.run.results {
            let benchmark_name = result.benchmark.name;
            let time: humantime::Duration = Duration::from_secs_f64(result.time).into();
            let time_text = style(format!("{}", time)).bold();

            if output_json {
                log_json!(format!(
                    "{{\"event\": \"benchmark_ran\", \"name\": \"{}\", \"time\": \"{}\"}}",
                    benchmark_name, result.time,
                ));
            }
            info!("{} - Time: {}", benchmark_name, time_text);
        }
        end_group!();
    }

    Ok(())
}
