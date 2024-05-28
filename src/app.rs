use crate::{auth, prelude::*, run};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
struct Cli {
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

    match cli.command {
        Commands::Run(args) => run::run(args).await?,
        Commands::Auth(args) => auth::run(args).await?,
    }
    Ok(())
}
