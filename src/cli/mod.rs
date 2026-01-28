mod auth;
pub(crate) mod exec;
pub(crate) mod run;
mod setup;
mod shared;
mod show;
mod use_mode;

pub(crate) use shared::*;

use std::path::PathBuf;

use crate::{
    api_client::CodSpeedAPIClient,
    config::CodSpeedConfig,
    local_logger::{CODSPEED_U8_COLOR_CODE, init_local_logger},
    prelude::*,
    project_config::ProjectConfig,
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

    /// The configuration name to use
    /// If provided, the configuration will be loaded from ~/.config/codspeed/{config-name}.yaml
    /// Otherwise, loads from ~/.config/codspeed/config.yaml
    #[arg(long, env = "CODSPEED_CONFIG_NAME", global = true)]
    pub config_name: Option<String>,

    /// Path to project configuration file (codspeed.yaml)
    /// If provided, loads config from this path. Otherwise, searches for config files
    /// in the current directory and upward to the git root.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

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
    /// Run a benchmark program that already contains the CodSpeed instrumentation and upload the results to CodSpeed
    #[command(alias = "r")]
    Run(Box<run::RunArgs>),
    /// Run a command after adding CodSpeed instrumentation to it and upload the results to
    /// CodSpeed
    #[command(alias = "x")]
    Exec(Box<exec::ExecArgs>),
    /// Manage the CLI authentication state
    Auth(auth::AuthArgs),
    /// Pre-install the codspeed executors
    Setup,
    /// Set the codspeed mode for the rest of the shell session
    Use(use_mode::UseArgs),
    /// Show the codspeed mode previously set in this shell session with `codspeed use`
    Show,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let codspeed_config =
        CodSpeedConfig::load_with_override(cli.config_name.as_deref(), cli.oauth_token.as_deref())?;
    let api_client = CodSpeedAPIClient::try_from((&cli, &codspeed_config))?;

    // Discover project configuration file (this may change the working directory)
    let project_config =
        ProjectConfig::discover_and_load(cli.config.as_deref(), &std::env::current_dir()?)?;

    // In the context of the CI, it is likely that a ~ made its way here without being expanded by the shell
    let setup_cache_dir = cli
        .setup_cache_dir
        .as_ref()
        .map(|d| PathBuf::from(shellexpand::tilde(d).as_ref()));
    let setup_cache_dir = setup_cache_dir.as_deref();

    match cli.command {
        Commands::Run(_) | Commands::Exec(_) => {} // Run and Exec are responsible for their own logger initialization
        _ => {
            init_local_logger()?;
        }
    }

    match cli.command {
        Commands::Run(args) => {
            run::run(
                *args,
                &api_client,
                &codspeed_config,
                project_config.as_ref(),
                setup_cache_dir,
            )
            .await?
        }
        Commands::Exec(args) => {
            exec::run(
                *args,
                &api_client,
                &codspeed_config,
                project_config.as_ref(),
                setup_cache_dir,
            )
            .await?
        }
        Commands::Auth(args) => auth::run(args, &api_client, cli.config_name.as_deref()).await?,
        Commands::Setup => setup::setup(setup_cache_dir).await?,
        Commands::Use(args) => use_mode::run(args)?,
        Commands::Show => show::run()?,
    }
    Ok(())
}

impl Cli {
    /// Create a test CLI instance with a custom API URL for use in tests
    #[cfg(test)]
    pub fn test_with_url(api_url: String) -> Self {
        Self {
            api_url,
            oauth_token: None,
            config_name: None,
            config: None,
            setup_cache_dir: None,
            command: Commands::Setup,
        }
    }
}
