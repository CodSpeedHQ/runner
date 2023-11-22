use crate::{config::Config, prelude::*, runner, uploader, VERSION};
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
    show_banner();
    debug!("config: {:#?}", config);
    let run_data = runner::run(&config)?;
    if !config.skip_upload {
        uploader::upload(&config, &run_data).await?;
    }
    Ok(())
}
