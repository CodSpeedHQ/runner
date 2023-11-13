use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadMetadata {
    pub version: Option<u32>,
    pub tokenless: bool,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub owner: String,
    pub repository: String,
    pub commit_hash: String,
    pub event: String,
    pub profile_md5: String,
    pub gh_data: GhData,
    pub runner: Runner,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GhData {
    pub run_id: u32,
    pub job: String,
    pub sender: Option<Sender>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub id: u32,
    pub login: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PostResponse {
    pub status: String,
    pub upload_url: String,
    pub run_id: String,
}
