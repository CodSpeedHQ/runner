use crate::prelude::*;
use crate::{app::Cli, config::CodSpeedConfig};
use gql_client::{Client as GQLClient, ClientConfig};
use nestify::nest;
use serde::{Deserialize, Serialize};

pub struct CodSpeedAPIClient {
    pub gql_client: GQLClient,
}

impl TryFrom<&Cli> for CodSpeedAPIClient {
    type Error = Error;
    fn try_from(args: &Cli) -> Result<Self> {
        let codspeed_config = CodSpeedConfig::load()?;

        Ok(Self {
            gql_client: build_gql_api_client(&codspeed_config, args.api_url.clone()),
        })
    }
}

fn build_gql_api_client(codspeed_config: &CodSpeedConfig, api_url: String) -> GQLClient {
    let headers = match &codspeed_config.auth.token {
        Some(token) => {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Authorization".to_string(), token.to_string());
            headers
        }
        None => Default::default(),
    };

    GQLClient::new_with_config(ClientConfig {
        endpoint: api_url,
        timeout: Some(10),
        headers: Some(headers),
        proxy: None,
    })
}

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct CreateLoginSessionData {
        create_login_session: pub struct CreateLoginSessionPayload {
            pub callback_url: String,
            pub session_id: String,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsumeLoginSessionVars {
    session_id: String,
}
nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct ConsumeLoginSessionData {
        consume_login_session: pub struct ConsumeLoginSessionPayload {
            pub token: Option<String>
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FetchLocalRunReportVars {
    pub owner: String,
    pub name: String,
    pub run_id: String,
}
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum ReportConclusion {
    AcknowledgedFailure,
    Failure,
    MissingBaseRun,
    Success,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchLocalRunReportHeadReport {
    pub id: String,
    pub impact: Option<f64>,
    pub conclusion: ReportConclusion,
}
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunStatus {
    Pending,
    Processing,
    Completed,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchLocalRunReportRun {
    pub id: String,
    pub status: RunStatus,
    pub url: String,
    pub head_reports: Vec<FetchLocalRunReportHeadReport>,
}
nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct FetchLocalRunReportData {
        repository: pub struct FetchLocalRunReportRepository {
            pub runs: Vec<FetchLocalRunReportRun>,
        }
    }
}

impl CodSpeedAPIClient {
    pub async fn create_login_session(&self) -> Result<CreateLoginSessionPayload> {
        let response = self
            .gql_client
            .query_unwrap::<CreateLoginSessionData>(include_str!("queries/CreateLoginSession.gql"))
            .await;
        match response {
            Ok(response) => Ok(response.create_login_session),
            Err(err) => bail!("Failed to create login session: {}", err),
        }
    }

    pub async fn consume_login_session(
        &self,
        session_id: &str,
    ) -> Result<ConsumeLoginSessionPayload> {
        let response = self
            .gql_client
            .query_with_vars_unwrap::<ConsumeLoginSessionData, ConsumeLoginSessionVars>(
                include_str!("queries/ConsumeLoginSession.gql"),
                ConsumeLoginSessionVars {
                    session_id: session_id.to_string(),
                },
            )
            .await;
        match response {
            Ok(response) => Ok(response.consume_login_session),
            Err(err) => bail!("Failed to use login session: {}", err),
        }
    }

    pub async fn fetch_local_run_report(
        &self,
        vars: FetchLocalRunReportVars,
    ) -> Result<FetchLocalRunReportRun> {
        let response = self
            .gql_client
            .query_with_vars_unwrap::<FetchLocalRunReportData, FetchLocalRunReportVars>(
                include_str!("queries/FetchLocalRunReport.gql"),
                vars.clone(),
            )
            .await;
        match response {
            Ok(response) => match response.repository.runs.into_iter().next() {
                Some(run) => Ok(run),
                None => bail!(
                    "No runs found for owner: {}, name: {}, run_id: {}",
                    vars.owner,
                    vars.name,
                    vars.run_id
                ),
            },
            Err(err) => bail!("Failed to fetch local run report: {}", err),
        }
    }
}
