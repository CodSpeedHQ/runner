use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RepositoryProvider {
    GitLab,
    GitHub,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunEnvironment {
    GithubActions,
    GitlabCi,
    Buildkite,
    Local,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RunEnvironmentMetadata {
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub id: String,
    pub login: String,
}
