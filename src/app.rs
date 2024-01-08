use std::env;

use crate::{
    ci_provider, config::Config, instruments::Instruments, prelude::*, runner, uploader, VERSION,
};
use clap::Parser;

fn show_banner() {
    let banner = format!(
        r#"
   ______            __ _____                         __
  / ____/____   ____/ // ___/ ____   ___   ___   ____/ /
 / /    / __ \ / __  / \__ \ / __ \ / _ \ / _ \ / __  /
/ /___ / /_/ // /_/ / ___/ // /_/ //  __//  __// /_/ /
\____/ \____/ \__,_/ /____// .___/ \___/ \___/ \__,_/
  https://codspeed.io     /_/          runner v{}
"#,
        VERSION
    );
    println!("{}", banner);
    debug!("codspeed-runner v{}", VERSION);
}

#[derive(Parser, Debug)]
pub struct AppArgs {
    /// The upload URL to use for uploading the results, useful for on-premises installations
    #[arg(long)]
    pub upload_url: Option<String>,

    /// The token to use for uploading the results,
    /// if not provided it will be read from the CODSPEED_TOKEN environment variable
    #[arg(long)]
    pub token: Option<String>,

    /// The directory where the command will be executed.
    #[arg(long)]
    pub working_directory: Option<String>,

    /// The name of the environment variable that contains the MongoDB URI to patch,
    /// if not provided it will be read from the CODSPEED_MONGO_INSTR_URI_ENV_NAME environment variable
    #[arg(long)]
    pub mongo_uri_env_name: Option<String>,

    /// Only for debugging purposes, skips the upload of the results
    #[arg(long, default_value = "false", hide = true)]
    pub skip_upload: bool,

    /// Only for debugging purposes, skips the setup of the runner
    #[arg(long, default_value = "false", hide = true)]
    pub skip_setup: bool,

    /// The bench command to run
    pub command: Vec<String>,
}

pub async fn run() -> Result<()> {
    let args = AppArgs::parse();
    let config = Config::try_from(args)?;
    let provider = ci_provider::get_provider(&config)?;
    let instruments = Instruments::from(&config);

    let log_level = env::var("CODSPEED_LOG")
        .ok()
        .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Info);
    log::set_max_level(log_level);
    provider.setup_logger()?;

    show_banner();
    debug!("config: {:#?}", config);

    let run_data = runner::run(&config, &instruments).await?;
    if !config.skip_upload {
        start_group!("Upload the results");
        uploader::upload(&config, provider, &run_data, &instruments).await?;
        end_group!();
    }
    Ok(())
}
