use serde::{Deserialize, Serialize};

use crate::run::{
    check_system::SystemInfo,
    ci_provider::interfaces::{CIProviderMetadata, RepositoryProvider},
    instruments::InstrumentName,
    runner::ExecutorName,
};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadMetadata {
    pub repository_provider: RepositoryProvider,
    pub version: Option<u32>,
    pub tokenless: bool,
    pub profile_md5: String,
    pub runner: Runner,
    pub platform: String,
    pub commit_hash: String,
    #[serde(flatten)]
    pub ci_provider_metadata: CIProviderMetadata,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
    pub name: String,
    pub version: String,
    pub instruments: Vec<InstrumentName>,
    pub executor: ExecutorName,
    #[serde(flatten)]
    pub system_info: SystemInfo,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    pub status: String,
    pub upload_url: String,
    pub run_id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadError {
    pub error: String,
}
