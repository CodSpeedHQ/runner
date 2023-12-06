use serde::{Deserialize, Serialize};

use crate::ci_provider::interfaces::ProviderMetadata;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadMetadata {
    pub version: Option<u32>,
    pub tokenless: bool,
    pub profile_md5: String,
    pub runner: Runner,
    pub platform: String,
    #[serde(flatten)]
    pub provider_metadata: ProviderMetadata,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    pub status: String,
    pub upload_url: String,
    pub run_id: String,
}
