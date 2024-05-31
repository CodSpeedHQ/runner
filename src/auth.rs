use std::time::Duration;

use crate::logger::get_local_logger;
use crate::{api_client::CodSpeedAPIClient, config::Config, prelude::*};
use clap::{Args, Subcommand};
use simplelog::CombinedLogger;
use tokio::time::{sleep, Instant};

#[derive(Debug, Args)]
pub struct AuthArgs {
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

pub async fn run(args: AuthArgs, api_client: &CodSpeedAPIClient) -> Result<()> {
    init_logger()?;

    match args.command {
        AuthCommands::Login => login(api_client).await?,
    }
    Ok(())
}

const LOGIN_SESSION_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes

async fn login(api_client: &CodSpeedAPIClient) -> Result<()> {
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
