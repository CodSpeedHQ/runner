use crate::run::{
    check_system::SystemInfo,
    config::Config,
    run_environment::{RunEnvironment, RunEnvironmentProvider},
    runner::ExecutorName,
    runner::RunData,
    uploader::UploadError,
};
use crate::{prelude::*, request_client::REQUEST_CLIENT};
use async_compression::tokio::write::GzipEncoder;
use base64::{Engine as _, engine::general_purpose};
use console::style;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;
use tokio_tar::Builder;

use super::interfaces::{UploadData, UploadMetadata};

/// Create a tar.gz archive buffer of the profile folder and return its md5 hash encoded in base64
async fn get_profile_archive_buffer(run_data: &RunData) -> Result<(Vec<u8>, String)> {
    let enc = GzipEncoder::new(Vec::new());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(".", run_data.profile_folder.clone())
        .await?;
    let mut gzip_encoder = tar.into_inner().await?;
    gzip_encoder.shutdown().await?;

    let archive_buffer = gzip_encoder.into_inner();
    let archive_digest = md5::compute(archive_buffer.as_slice());
    let archive_hash = general_purpose::STANDARD.encode(archive_digest.0);

    Ok((archive_buffer, archive_hash))
}

async fn retrieve_upload_data(
    config: &Config,
    upload_metadata: &UploadMetadata,
) -> Result<UploadData> {
    let mut upload_request = REQUEST_CLIENT
        .post(config.upload_url.clone())
        .json(&upload_metadata);
    if !upload_metadata.tokenless {
        upload_request = upload_request.header("Authorization", config.token.clone().unwrap());
    }

    let response = upload_request.send().await;

    match response {
        Ok(response) => {
            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await?;
                let mut error_message = serde_json::from_str::<UploadError>(&text)
                    .map(|body| body.error)
                    .unwrap_or(text);
                if status == StatusCode::UNAUTHORIZED {
                    let additional_message =
                        if upload_metadata.run_environment == RunEnvironment::Local {
                            "Run `codspeed auth login` to authenticate the CLI"
                        } else {
                            "Check that CODSPEED_TOKEN is set and has the correct value"
                        };
                    error_message.push_str(&format!("\n\n{additional_message}"));
                }

                debug!(
                    "Check that owner and repository are correct (case-sensitive!): {}/{}",
                    upload_metadata.run_environment_metadata.owner,
                    upload_metadata.run_environment_metadata.repository
                );

                bail!(
                    "Failed to retrieve upload data: {}\n  -> {} {}",
                    status,
                    style("Reason:").bold(),
                    // we have to manually apply the style to the error message, because nesting styles is not supported by the console crate: https://github.com/console-rs/console/issues/106
                    style(error_message).red()
                );
            }

            Ok(response.json().await?)
        }
        Err(err) => Err(err.into()),
    }
}

async fn upload_archive_buffer(
    upload_data: &UploadData,
    archive_buffer: Vec<u8>,
    archive_hash: &String,
) -> Result<()> {
    let response = REQUEST_CLIENT
        .put(upload_data.upload_url.clone())
        .header("Content-Type", "application/gzip")
        .header("Content-Length", archive_buffer.len())
        .header("Content-MD5", archive_hash)
        .body(archive_buffer)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        bail!(
            "Failed to upload performance report: {}\n  -> {} {}",
            status,
            style("Reason:").bold(),
            style(error_text).red()
        );
    }

    Ok(())
}

pub struct UploadResult {
    pub run_id: String,
}

#[allow(clippy::borrowed_box)]
pub async fn upload(
    config: &Config,
    system_info: &SystemInfo,
    provider: &Box<dyn RunEnvironmentProvider>,
    run_data: &RunData,
    executor_name: ExecutorName,
) -> Result<UploadResult> {
    let (archive_buffer, archive_hash) = get_profile_archive_buffer(run_data).await?;

    debug!(
        "Run Environment provider detected: {:?}",
        provider.get_run_environment()
    );

    let upload_metadata =
        provider.get_upload_metadata(config, system_info, &archive_hash, executor_name)?;
    debug!("Upload metadata: {upload_metadata:#?}");
    info!(
        "Linked repository: {}\n",
        style(format!(
            "{}/{}",
            upload_metadata.run_environment_metadata.owner,
            upload_metadata.run_environment_metadata.repository
        ))
        .bold(),
    );
    if upload_metadata.tokenless {
        let hash = upload_metadata.get_hash();
        info!("CodSpeed Run Hash: \"{hash}\"");
    }

    info!("Preparing upload...");
    let upload_data = retrieve_upload_data(config, &upload_metadata).await?;
    debug!("runId: {}", upload_data.run_id);

    info!("Uploading performance data...");
    debug!("Uploading {} bytes...", archive_buffer.len());
    upload_archive_buffer(&upload_data, archive_buffer, &archive_hash).await?;
    info!("Performance data uploaded");

    Ok(UploadResult {
        run_id: upload_data.run_id,
    })
}

#[cfg(test)]
mod tests {
    use temp_env::async_with_vars;
    use url::Url;

    use super::*;
    use std::path::PathBuf;

    // TODO: remove the ignore when implementing network mocking
    #[ignore]
    #[tokio::test]
    async fn test_upload() {
        let config = Config {
            command: "pytest tests/ --codspeed".into(),
            upload_url: Url::parse("change me").unwrap(),
            token: Some("change me".into()),
            ..Config::test()
        };
        let run_data = RunData {
            profile_folder: PathBuf::from(format!(
                "{}/src/uploader/samples/adrien-python-test",
                env!("CARGO_MANIFEST_DIR")
            )),
        };
        let system_info = SystemInfo::test();
        async_with_vars(
            [
                ("GITHUB_ACTIONS", Some("true")),
                ("GITHUB_ACTOR_ID", Some("19605940")),
                ("GITHUB_ACTOR", Some("adriencaccia")),
                ("GITHUB_BASE_REF", Some("main")),
                ("GITHUB_EVENT_NAME", Some("pull_request")),
                (
                    "GITHUB_EVENT_PATH",
                    Some(
                        format!(
                            "{}/src/uploader/samples/pr-event.json",
                            env!("CARGO_MANIFEST_DIR")
                        )
                        .as_str(),
                    ),
                ),
                ("GITHUB_HEAD_REF", Some("feat/codspeed-runner")),
                ("GITHUB_JOB", Some("log-env")),
                ("GITHUB_REF", Some("refs/pull/22/merge")),
                ("GITHUB_REPOSITORY", Some("my-org/adrien-python-test")),
                ("GITHUB_RUN_ID", Some("6957110437")),
                (
                    "GITHUB_SHA",
                    Some("5bd77cb0da72bef094893ed45fb793ff16ecfbe3"),
                ),
                ("VERSION", Some("0.1.0")),
            ],
            async {
                let provider = crate::run::run_environment::get_provider(&config).unwrap();
                upload(
                    &config,
                    &system_info,
                    &provider,
                    &run_data,
                    ExecutorName::Valgrind,
                )
                .await
                .unwrap();
            },
        )
        .await;
    }
}
