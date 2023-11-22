use async_compression::tokio::write::GzipEncoder;
use base64::{engine::general_purpose, Engine as _};
use lazy_static::lazy_static;
use tokio::io::AsyncWriteExt;
use tokio_tar::Builder;

use crate::{config::Config, prelude::*, runner::RunData};
use reqwest::ClientBuilder;
use reqwest_middleware::{ClientBuilder as ClientWithMiddlewareBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde_json::json;

use super::{
    ci_provider::CIProvider,
    github_actions_provider::GitHubActionsProvider,
    interfaces::{UploadData, UploadMetadata},
};

fn get_provider(config: &Config) -> Result<impl CIProvider> {
    if GitHubActionsProvider::detect() {
        let provider = GitHubActionsProvider::try_from(config)?;
        return Ok(provider);
    }

    bail!("No CI provider detected")
}

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

const UPLOAD_RETRY_COUNT: u32 = 3;

lazy_static! {
    static ref UPLOAD_REQUEST_CLIENT: ClientWithMiddleware = ClientWithMiddlewareBuilder::new(
        ClientBuilder::new()
            .user_agent("codspeed-runner")
            .build()
            .unwrap()
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(UPLOAD_RETRY_COUNT)
    ))
    .build();
}

async fn retrieve_upload_data(
    config: &Config,
    upload_metadata: &UploadMetadata,
) -> Result<UploadData> {
    let mut upload_request = UPLOAD_REQUEST_CLIENT
        .post(config.upload_url.clone())
        .json(&upload_metadata);
    if !upload_metadata.tokenless {
        upload_request = upload_request.header("Authorization", config.token.clone().unwrap());
    }

    let response = upload_request.send().await?.json::<UploadData>().await?;

    Ok(response)
}

async fn upload_archive_buffer(
    upload_data: &UploadData,
    archive_buffer: Vec<u8>,
    archive_hash: &String,
) -> Result<()> {
    UPLOAD_REQUEST_CLIENT
        .put(upload_data.upload_url.clone())
        .header("Content-Type", "application/gzip")
        .header("Content-Length", archive_buffer.len())
        .header("Content-MD5", archive_hash)
        .body(archive_buffer)
        .send()
        .await?;

    Ok(())
}

pub async fn upload(config: &Config, run_data: &RunData) -> Result<()> {
    let (archive_buffer, archive_hash) = get_profile_archive_buffer(run_data).await?;

    let provider = get_provider(config)?;
    debug!("CI provider detected: {:#?}", provider.get_provider_name());

    let upload_metadata = provider.get_upload_metadata(config, &archive_hash)?;
    debug!("Upload metadata: {:#?}", upload_metadata);
    if upload_metadata.tokenless {
        let hash = sha256::digest(json!(upload_metadata).to_string());
        info!("CodSpeed Run Hash: {}", hash);
    }

    info!("Preparing upload...");
    let upload_data = retrieve_upload_data(config, &upload_metadata).await?;
    debug!("runId: {}", upload_data.run_id);

    info!("Uploading profile data...");
    debug!("Uploading {} bytes...", archive_buffer.len());
    upload_archive_buffer(&upload_data, archive_buffer, &archive_hash).await?;
    info!("Results uploaded.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use temp_env::async_with_vars;
    use url::Url;

    use super::*;
    use crate::runner::RunData;
    use std::path::PathBuf;

    // TODO: remove the ignore when implementing network mocking
    #[ignore]
    #[tokio::test]
    async fn test_upload() {
        let config = Config {
            command: "pytest tests/ --codspeed".into(),
            upload_url: Url::parse("change me").unwrap(),
            skip_setup: false,
            skip_upload: false,
            token: Some("change me".into()),
            working_directory: None,
        };
        let run_data = RunData {
            profile_folder: PathBuf::from(format!(
                "{}/src/uploader/samples/adrien-python-test",
                env!("CARGO_MANIFEST_DIR")
            )),
        };
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
                upload(&config, &run_data).await.unwrap();
            },
        )
        .await;
    }
}
