use crate::{
    api_client::CodSpeedAPIClient,
    auth,
    local_logger::{init_local_logger, CODSPEED_U8_COLOR_CODE},
    prelude::*,
    run, setup,
};
use clap::{
    builder::{styling, Styles},
    Parser, Subcommand,
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
    let api_client = CodSpeedAPIClient::try_from(&cli)?;

    match cli.command {
        Commands::Run(_) => {} // Run is responsible for its own logger initialization
        _ => {
            init_local_logger()?;
        }
    }

    match cli.command {
        Commands::Run(args) => run::run(args, &api_client).await?,
        Commands::Auth(args) => auth::run(args, &api_client).await?,
        Commands::Setup => setup::setup().await?,
    }
    Ok(())
}
