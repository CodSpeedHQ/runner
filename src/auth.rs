use std::time::Duration;

use crate::{config::Config, logger::get_local_logger, prelude::*};
use clap::{Args, Subcommand};
use gql_client::{Client as GQLClient, ClientConfig};
use nestify::nest;
use serde::{Deserialize, Serialize};
use simplelog::CombinedLogger;
use tokio::time::{sleep, Instant};

#[derive(Debug, Args)]
pub struct AuthArgs {
    /// The URL of the CodSpeed GraphQL API
    #[arg(long, env = "CODSPEED_API_URL", global = true, hide = true)]
    api_url: Option<String>,

    #[command(subcommand)]
    command: AuthCommands,
}

#[derive(Debug, Subcommand)]
enum AuthCommands {
    /// Login to CodSpeed
    Login,
}

// TODO: tweak the logger to make it more user-friendly
fn init_logger() -> Result<()> {
    let logger = get_local_logger();
    CombinedLogger::init(vec![logger])?;
    Ok(())
}

pub async fn run(args: AuthArgs) -> Result<()> {
    init_logger()?;
    let api_client = CodSpeedAPIClient::from(&args);

    match args.command {
        AuthCommands::Login => login(api_client).await?,
    }
    Ok(())
}

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct CreateLoginSessionData {
        create_login_session: struct CreateLoginSessionPayload {
            callback_url: String,
            session_id: String,
        }
    }
}

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct ConsumeLoginSessionData {
        consume_login_session: struct ConsumeLoginSessionPayload {
            token: Option<String>
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsumeLoginSessionVars {
    session_id: String,
}

struct CodSpeedAPIClient {
    gql_client: GQLClient,
}

impl From<&AuthArgs> for CodSpeedAPIClient {
    fn from(args: &AuthArgs) -> Self {
        Self {
            gql_client: build_gql_api_client(args.api_url.clone()),
        }
    }
}

const CODSPEED_GRAPHQL_ENDPOINT: &str = "https://gql.codspeed.io/";

fn build_gql_api_client(api_url: Option<String>) -> GQLClient {
    let endpoint = api_url.unwrap_or_else(|| CODSPEED_GRAPHQL_ENDPOINT.to_string());

    GQLClient::new_with_config(ClientConfig {
        endpoint,
        timeout: Some(10),
        headers: Default::default(),
        proxy: None,
    })
}

impl CodSpeedAPIClient {
    async fn create_login_session(&self) -> Result<CreateLoginSessionPayload> {
        let response = self
            .gql_client
            .query_unwrap::<CreateLoginSessionData>(include_str!("queries/CreateLoginSession.gql"))
            .await;
        match response {
            Ok(response) => Ok(response.create_login_session),
            Err(err) => bail!("Failed to create login session: {}", err),
        }
    }

    async fn consume_login_session(&self, session_id: &str) -> Result<ConsumeLoginSessionPayload> {
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
}

const LOGIN_SESSION_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes

async fn login(api_client: CodSpeedAPIClient) -> Result<()> {
    debug!("Login to CodSpeed");
    debug!("Creating login session...");
    let login_session_payload = api_client.create_login_session().await?;
    info!(
        "Login session created, open the following URL in your browser: {}",
        login_session_payload.callback_url
    );

    info!("Waiting for the login to be completed...");
    let token;
    let start = Instant::now();
    loop {
        if LOGIN_SESSION_MAX_DURATION < start.elapsed() {
            bail!("Login session expired, please try again");
        }

        match api_client
            .consume_login_session(&login_session_payload.session_id)
            .await?
            .token
        {
            Some(token_from_api) => {
                token = token_from_api;
                break;
            }
            None => sleep(Duration::from_secs(5)).await,
        }
    }
    debug!("Login completed");

    let mut config = Config::load().await?;
    config.auth.token = token;
    config.persist().await?;
    debug!("Token saved to configuration file");

    info!("Login successful, your are now authenticated on CodSpeed");

    Ok(())
}
