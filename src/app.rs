use std::path::PathBuf;

use crate::{
    api_client::CodSpeedAPIClient,
    auth,
    config::{CodSpeedConfig, DEFAULT_API_URL, DEFAULT_PROFILE_NAME},
    local_logger::{CODSPEED_U8_COLOR_CODE, init_local_logger},
    prelude::*,
    run, setup,
};
use clap::{
    Parser, Subcommand,
    builder::{Styles, styling},
};

fn create_styles() -> Styles {
    styling::Styles::styled()
        .header(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .usage(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .literal(
            styling::Ansi256Color(CODSPEED_U8_COLOR_CODE).on_default() | styling::Effects::BOLD,
        )
        .placeholder(styling::AnsiColor::Cyan.on_default())
}

#[derive(Parser, Debug)]
#[command(version, about = "The CodSpeed CLI tool", styles = create_styles())]
pub struct Cli {
    /// The URL of the CodSpeed GraphQL API
    #[arg(
        long,
        env = "CODSPEED_API_URL",
        global = true,
        hide = true,
        default_value = "https://gql.codspeed.io/"
    )]
    pub api_url: String,

    /// The OAuth token to use for all requests
    #[arg(long, env = "CODSPEED_OAUTH_TOKEN", global = true, hide = true)]
    pub oauth_token: Option<String>,

    /// The profile to use for authentication and API configuration
    #[arg(
        long,
        env = "CODSPEED_PROFILE",
        global = true,
        default_value = DEFAULT_PROFILE_NAME
    )]
    pub profile: String,

    /// The upload URL for uploading results, useful for on-premises installations
    #[arg(long, env = "CODSPEED_UPLOAD_URL", global = true, hide = true)]
    pub upload_url: Option<String>,

    /// The directory to use for caching installed tools
    /// The runner will restore cached tools from this directory before installing them.
    /// After successful installation, the runner will cache the installed tools to this directory.
    /// Only supported on ubuntu and debian systems.
    #[arg(long, env = "CODSPEED_SETUP_CACHE_DIR", global = true)]
    pub setup_cache_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the bench command and upload the results to CodSpeed
    Run(run::RunArgs),
    /// Manage the CLI authentication state
    Auth(auth::AuthArgs),
    /// Pre-install the codspeed executors
    Setup,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let codspeed_config =
        CodSpeedConfig::load_with_override(&cli.profile, cli.oauth_token.as_deref())?;

    // Resolve the effective API URL: CLI/env override > profile > default
    let effective_api_url = if cli.api_url != DEFAULT_API_URL {
        // User explicitly provided an API URL via CLI or env
        cli.api_url.clone()
    } else {
        // Use profile's API URL or default
        codspeed_config.resolve_api_url(&cli.profile)
    };

    let api_client = CodSpeedAPIClient::try_from((&cli, &codspeed_config, effective_api_url))?;
    // In the context of the CI, it is likely that a ~ made its way here without being expanded by the shell
    let setup_cache_dir = cli
        .setup_cache_dir
        .as_ref()
        .map(|d| PathBuf::from(shellexpand::tilde(d).as_ref()));
    let setup_cache_dir = setup_cache_dir.as_deref();

    match cli.command {
        Commands::Run(_) => {} // Run is responsible for its own logger initialization
        _ => {
            init_local_logger()?;
        }
    }

    match &cli.command {
        Commands::Run(args) => {
            run::run(
                args.clone(),
                &api_client,
                &codspeed_config,
                setup_cache_dir,
                &cli.profile,
                cli.upload_url.clone(),
            )
            .await?
        }
        Commands::Auth(args) => {
            auth::run(args.clone(), &api_client, &cli, &codspeed_config).await?
        }
        Commands::Setup => setup::setup(setup_cache_dir).await?,
    }
    Ok(())
}
