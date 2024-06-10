use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::prelude::*;
use crate::run::{config::Config, logger::Logger};
use crate::VERSION;
use clap::Args;

mod check_system;
pub mod ci_provider;
mod helpers;
mod instruments;
mod poll_results;
mod runner;
mod uploader;

pub mod config;
pub mod logger;

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
    debug!("codspeed v{}", VERSION);
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The upload URL to use for uploading the results, useful for on-premises installations
    #[arg(long)]
    pub upload_url: Option<String>,

    /// The token to use for uploading the results,
    #[arg(long, env = "CODSPEED_TOKEN")]
    pub token: Option<String>,

    /// The directory where the command will be executed.
    #[arg(long)]
    pub working_directory: Option<String>,

    /// Comma-separated list of instruments to enable. Possible values: mongodb.
    #[arg(long, value_delimiter = ',')]
    pub instruments: Vec<String>,

    /// The name of the environment variable that contains the MongoDB URI to patch.
    /// If not provided, user will have to provide it dynamically through a CodSpeed integration.
    ///
    /// Only used if the `mongodb` instrument is enabled.
    #[arg(long)]
    pub mongo_uri_env_name: Option<String>,

    /// Only for debugging purposes, skips the upload of the results
    #[arg(
        long,
        default_value = "false",
        hide = true,
        env = "CODSPEED_SKIP_UPLOAD"
    )]
    pub skip_upload: bool,

    /// Only for debugging purposes, skips the setup of the runner
    #[arg(long, default_value = "false", hide = true)]
    pub skip_setup: bool,

    /// The bench command to run
    pub command: Vec<String>,
}

#[cfg(test)]
impl RunArgs {
    /// Constructs a new `RunArgs` with default values for testing purposes
    pub fn test() -> Self {
        Self {
            upload_url: None,
            token: None,
            working_directory: None,
            instruments: vec![],
            mongo_uri_env_name: None,
            skip_upload: false,
            skip_setup: false,
            command: vec![],
        }
    }
}

pub async fn run(args: RunArgs, api_client: &CodSpeedAPIClient) -> Result<()> {
    let mut config = Config::try_from(args)?;
    let provider = ci_provider::get_provider(&config)?;
    let codspeed_config = CodSpeedConfig::load()?;
    let logger = Logger::new(&provider)?;

    show_banner();
    debug!("config: {:#?}", config);

    if provider.get_provider_slug() == "local" {
        if codspeed_config.auth.token.is_none() {
            bail!("You have to authenticate the CLI first. Run `codspeed auth login`.");
        }
        debug!("Using the token from the CodSpeed configuration file");
        config.set_token(codspeed_config.auth.token.clone());
    }

    let system_info = check_system::check_system()?;
    let run_data = runner::run(&config, &system_info).await?;

    if !config.skip_upload {
        start_group!("Upload the results");
        logger.persist_log_to_profile_folder(&run_data)?;
        let upload_result = uploader::upload(&config, &system_info, &provider, &run_data).await?;
        end_group!();

        if provider.get_provider_slug() == "local" {
            start_group!("Fetch the results");
            poll_results::poll_results(api_client, &provider, upload_result.run_id).await?;
            end_group!();
        }
    }

    Ok(())
}
