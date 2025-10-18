use std::time::Duration;

use crate::{api_client::CodSpeedAPIClient, config::CodSpeedConfig, prelude::*};
use clap::{Args, Subcommand};
use console::style;
use tokio::time::{Instant, sleep};

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

pub async fn run(args: AuthArgs, api_client: &CodSpeedAPIClient) -> Result<()> {
    match args.command {
        AuthCommands::Login => login(api_client).await?,
    }
    Ok(())
}

const LOGIN_SESSION_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes

async fn login(api_client: &CodSpeedAPIClient) -> Result<()> {
    debug!("Login to CodSpeed");
    start_group!("Creating login session");
    let login_session_payload = api_client.create_login_session().await?;
    end_group!();

    if open::that(&login_session_payload.callback_url).is_ok() {
        info!("Your browser has been opened to complete the login process");
    } else {
        warn!("Failed to open the browser automatically, please open the URL manually");
    }
    info!(
        "Authentication URL: {}\n",
        style(login_session_payload.callback_url)
            .blue()
            .bold()
            .underlined()
    );

    start_group!("Waiting for the login to be completed");
    let token;
    let start = Instant::now();
    loop {
        if start.elapsed() > LOGIN_SESSION_MAX_DURATION {
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
    end_group!();

    let mut config = CodSpeedConfig::load()?;
    config.auth.token = Some(token);
    config.persist()?;
    debug!("Token saved to configuration file");

    info!("Login successful, your are now authenticated on CodSpeed");

    Ok(())
}
