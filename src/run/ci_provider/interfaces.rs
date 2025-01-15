use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RepositoryProvider {
    GitLab,
    GitHub,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Platform {
    GithubActions,
    GitlabCi,
    Buildkite,
    Local,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CIProviderMetadata {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub head_ref: Option<String>,
    pub base_ref: Option<String>,
    pub owner: String,
    pub repository: String,
    pub event: RunEvent,
    pub sender: Option<Sender>,
    pub gh_data: Option<GhData>,
    pub gl_data: Option<GlData>,
    pub repository_root_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunEvent {
    Push,
    PullRequest,
    WorkflowDispatch,
    Schedule,
    Local,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GhData {
    pub run_id: String,
    pub job: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlData {
    pub run_id: String,
    pub job: String,
}

/// Each execution of the CLI maps to a `RunPart`.
///
/// Several `RunParts` can be aggregated in a single `Run` thanks to this data.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunPart {
    /// A unique identifier of the `Run` on the platform
    pub run_id: String,

    /// Uniquely identify a `RunPart` within a `Run`.
    ///
    /// This id can be the same between `RunParts` of different `Runs`.
    pub run_part_id: String,

    /// The name of the job. For example, on Github Actions, the workflow name.
    ///
    /// This is **not** unique between executions of the CLI, even between matrix jobs.
    pub job_name: String,

    /// Some relevant metadata.
    ///
    /// This can include matrix and strategy for GithubActions,
    /// some relevant env values.
    ///
    /// We use a `BTreeMap` and not a `HashMap` to keep insert order for
    /// `serde_json` serialization.
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub id: String,
    pub login: String,
}
