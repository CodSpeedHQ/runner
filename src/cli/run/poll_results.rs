use console::style;

use crate::api_client::CodSpeedAPIClient;
use crate::cli::run::helpers::benchmark_display::build_benchmark_table;
use crate::prelude::*;
use crate::upload::{UploadResult, poll_run_report};

#[allow(clippy::borrowed_box)]
pub async fn poll_results(
    api_client: &CodSpeedAPIClient,
    upload_result: &UploadResult,
    output_json: bool,
) -> Result<()> {
    let response = poll_run_report(api_client, upload_result).await?;

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

    if output_json {
        // TODO: Refactor `log_json` to avoid having to format the json manually
        // We could make use of structured logging for this https://docs.rs/log/latest/log/#structured-logging
        log_json!(format!(
            "{{\"event\": \"run_finished\", \"run_id\": \"{}\"}}",
            upload_result.run_id
        ));
    }

    if !response.run.results.is_empty() {
        end_group!();
        start_group!("Benchmark results");

        let table = build_benchmark_table(&response.run.results);
        info!("\n{table}");

        if output_json {
            for result in response.run.results {
                log_json!(format!(
                    "{{\"event\": \"benchmark_ran\", \"name\": \"{}\", \"time\": \"{}\"}}",
                    result.benchmark.name, result.value
                ));
            }
        }

        info!(
            "\nTo see the full report, visit: {}",
            style(response.run.url).blue().bold().underlined()
        );
    }

    Ok(())
}
