use crate::app::Cli;
use crate::prelude::*;
use gql_client::{Client as GQLClient, ClientConfig};
use nestify::nest;
use serde::{Deserialize, Serialize};

pub struct CodSpeedAPIClient {
    pub gql_client: GQLClient,
}

impl From<&Cli> for CodSpeedAPIClient {
    fn from(args: &Cli) -> Self {
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

nest! {
    #[derive(Debug, Deserialize, Serialize)]*
    #[serde(rename_all = "camelCase")]*
    struct ConsumeLoginSessionData {
        consume_login_session: pub struct ConsumeLoginSessionPayload {
            pub token: Option<String>
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsumeLoginSessionVars {
    session_id: String,
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
}
