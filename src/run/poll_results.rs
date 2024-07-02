use std::time::Duration;

use console::style;
use tokio::time::{sleep, Instant};
use url::Url;

use crate::api_client::{
    CodSpeedAPIClient, FetchLocalRunReportResponse, FetchLocalRunReportVars, RunStatus,
};
use crate::prelude::*;

use super::ci_provider::CIProvider;
use super::config::Config;

const RUN_PROCESSING_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes

#[allow(clippy::borrowed_box)]
pub async fn poll_results(
    config: &Config,
    api_client: &CodSpeedAPIClient,
    provider: &Box<dyn CIProvider>,
    run_id: String,
) -> Result<()> {
    let start = Instant::now();
    let provider_metadata = provider.get_provider_metadata()?;
    let owner = provider_metadata.owner;
    let name = provider_metadata.repository;
    let fetch_local_run_report_vars = FetchLocalRunReportVars {
        owner: owner.clone(),
        name: name.clone(),
        run_id: run_id.clone(),
    };

    let response;
    loop {
        if start.elapsed() > RUN_PROCESSING_MAX_DURATION {
            bail!("Polling results timed out");
        }

        match api_client
            .fetch_local_run_report(fetch_local_run_report_vars.clone())
            .await?
        {
            FetchLocalRunReportResponse { run, .. } if run.status != RunStatus::Completed => {
                sleep(Duration::from_secs(5)).await;
            }
            reponse_from_api => {
                response = reponse_from_api;
                break;
            }
        }
    }

    let report = response
        .run
        .head_reports
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No head report found in the run report"))?;

    info!("Report completed, here are the results:");
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
    }
    info!("Conclusion: {}", report.conclusion);

    let mut report_url = Url::parse(config.frontend_url.as_str())?;
    report_url.set_path(format!("{}/{}/runs/{}", owner, name, response.run.id).as_str());

    info!(
        "\nTo see the full report, visit: {}",
        style(report_url).blue().bold().underlined()
    );

    Ok(())
}
