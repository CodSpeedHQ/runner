use crate::{api_client::CodSpeedAPIClient, auth, prelude::*, run};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
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
    /// Commands related to authentication with CodSpeed
    Auth(auth::AuthArgs),
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let api_client = CodSpeedAPIClient::try_from(&cli)?;

    match cli.command {
        Commands::Run(args) => run::run(args, &api_client).await?,
        Commands::Auth(args) => auth::run(args, &api_client).await?,
    }
    Ok(())
}
