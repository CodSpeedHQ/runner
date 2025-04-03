use std::fmt::Display;

use crate::prelude::*;
use crate::{app::Cli, config::CodSpeedConfig};
use console::style;
use gql_client::{Client as GQLClient, ClientConfig};
use nestify::nest;
use serde::{Deserialize, Serialize};

pub struct CodSpeedAPIClient {
    gql_client: GQLClient,
    unauthenticated_gql_client: GQLClient,
}

impl TryFrom<(&Cli, &CodSpeedConfig)> for CodSpeedAPIClient {
    type Error = Error;
    fn try_from((args, codspeed_config): (&Cli, &CodSpeedConfig)) -> Result<Self> {
        Ok(Self {
            gql_client: build_gql_api_client(codspeed_config, args.api_url.clone(), true),
            unauthenticated_gql_client: build_gql_api_client(
                codspeed_config,
                args.api_url.clone(),
                false,
            ),
        })
    }
}

fn build_gql_api_client(
    codspeed_config: &CodSpeedConfig,
    api_url: String,
    with_auth: bool,
) -> GQLClient {
    let headers = if with_auth && codspeed_config.auth.token.is_some() {
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            codspeed_config.auth.token.clone().unwrap(),
        );
        headers
    } else {
        Default::default()
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

impl Display for ReportConclusion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportConclusion::AcknowledgedFailure => {
                write!(f, "{}", style("Acknowledged Failure").yellow().bold())
            }
            ReportConclusion::Failure => write!(f, "{}", style("Failure").red().bold()),
            ReportConclusion::MissingBaseRun => {
                write!(f, "{}", style("Missing Base Run").yellow().bold())
            }
            ReportConclusion::Success => write!(f, "{}", style("Success").green().bold()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchLocalRunReportRun {
    pub id: String,
    pub status: RunStatus,
    pub url: String,
    pub head_reports: Vec<FetchLocalRunReportHeadReport>,
    pub results: Vec<FetchLocalRunBenchmarkResult>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunStatus {
    Completed,
    Failure,
    Pending,
    Processing,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchLocalRunReportHeadReport {
    pub id: String,
    pub impact: Option<f64>,
    pub conclusion: ReportConclusion,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FetchLocalRunBenchmarkResult {
    pub time: f64,
    pub benchmark: FetchLocalRunBenchmark,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FetchLocalRunBenchmark {
    pub name: String,
}

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct FetchLocalRunReportData {
        repository: pub struct FetchLocalRunReportRepository {
            settings: struct FetchLocalRunReportSettings {
                allowed_regression: f64,
            },
            run: FetchLocalRunReportRun,
        }
    }
}

pub struct FetchLocalRunReportResponse {
    pub allowed_regression: f64,
    pub run: FetchLocalRunReportRun,
}

impl CodSpeedAPIClient {
    pub async fn create_login_session(&self) -> Result<CreateLoginSessionPayload> {
        let response = self
            .unauthenticated_gql_client
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
            .unauthenticated_gql_client
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
    ) -> Result<FetchLocalRunReportResponse> {
        let response = self
            .gql_client
            .query_with_vars_unwrap::<FetchLocalRunReportData, FetchLocalRunReportVars>(
                include_str!("queries/FetchLocalRunReport.gql"),
                vars.clone(),
            )
            .await;
        match response {
            Ok(response) => Ok(FetchLocalRunReportResponse {
                allowed_regression: response.repository.settings.allowed_regression,
                run: response.repository.run,
            }),
            Err(err) if err.contains_error_code("UNAUTHENTICATED") => {
                bail!("Your session has expired, please login again using `codspeed auth login`")
            }
            Err(err) => bail!("Failed to fetch local run report: {}", err),
        }
    }
}
