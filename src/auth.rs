use std::time::Duration;

use crate::{
    api_client::CodSpeedAPIClient,
    app::Cli,
    config::{CodSpeedConfig, DEFAULT_API_URL, DEFAULT_UPLOAD_URL},
    prelude::*,
};
use clap::{Args, Subcommand};
use console::style;
use tokio::time::{Instant, sleep};

#[derive(Debug, Args, Clone)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCommands,
}

#[derive(Debug, Subcommand, Clone)]
enum AuthCommands {
    /// Login to CodSpeed
    Login,
}

pub async fn run(
    args: AuthArgs,
    api_client: &CodSpeedAPIClient,
    cli: &Cli,
    config: &CodSpeedConfig,
) -> Result<()> {
    match args.command {
        AuthCommands::Login => login(api_client, cli, config).await?,
    }
    Ok(())
}

const LOGIN_SESSION_MAX_DURATION: Duration = Duration::from_secs(60 * 5); // 5 minutes

async fn login(api_client: &CodSpeedAPIClient, cli: &Cli, _config: &CodSpeedConfig) -> Result<()> {
    debug!("Login to CodSpeed with profile: {}", cli.profile);
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
    let profile = config.get_or_create_profile(&cli.profile);

    // Save the token
    profile.token = Some(token);

    // Only save URLs if they differ from defaults
    if cli.api_url != DEFAULT_API_URL {
        profile.api_url = Some(cli.api_url.clone());
        debug!("Saved custom API URL to profile: {}", cli.api_url);
    }

    if let Some(ref upload_url) = cli.upload_url {
        if upload_url != DEFAULT_UPLOAD_URL {
            profile.upload_url = Some(upload_url.clone());
            debug!("Saved custom upload URL to profile: {}", upload_url);
        }
    }

    config.persist()?;
    debug!(
        "Token saved to profile '{}' in configuration file",
        cli.profile
    );

    info!("Login successful, you are now authenticated on CodSpeed");
    if cli.profile != "default" {
        info!("Profile: {}", cli.profile);
    }

    Ok(())
}
