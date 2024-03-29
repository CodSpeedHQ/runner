use crate::{ci_provider, config::Config, logger::Logger, prelude::*, runner, uploader, VERSION};
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
impl AppArgs {
    /// Constructs a new `AppArgs` with default values for testing purposes
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

pub async fn run() -> Result<()> {
    let args = AppArgs::parse();
    let config = Config::try_from(args)?;
    let provider = ci_provider::get_provider(&config)?;
    let logger = Logger::new(&provider)?;

    show_banner();
    debug!("config: {:#?}", config);

    let run_data = runner::run(&config).await?;

    if !config.skip_upload {
        start_group!("Upload the results");
        logger.persist_log_to_profile_folder(&run_data)?;
        uploader::upload(&config, provider, &run_data).await?;
        end_group!();
    }
    Ok(())
}
