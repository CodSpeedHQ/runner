use std::env;

use crate::prelude::*;
use url::Url;

use crate::app::AppArgs;

#[derive(Debug)]
pub struct Config {
    pub upload_url: Url,
    pub token: Option<String>,
    pub working_directory: Option<String>,
    pub command: String,

    pub skip_upload: bool,
    pub skip_setup: bool,
}

#[cfg(test)]
impl Config {
    /// Constructs a new `Config` with default values for testing purposes
    pub fn test() -> Self {
        Self {
            upload_url: Url::parse(DEFAULT_UPLOAD_URL).unwrap(),
            token: None,
            working_directory: None,
            command: "".into(),
            skip_upload: false,
            skip_setup: false,
        }
    }
}

const DEFAULT_UPLOAD_URL: &str = "https://api.codspeed.io/upload";

impl TryFrom<AppArgs> for Config {
    type Error = Error;
    fn try_from(args: AppArgs) -> Result<Self> {
        let raw_upload_url = args.upload_url.unwrap_or_else(|| DEFAULT_UPLOAD_URL.into());
        let upload_url = Url::parse(&raw_upload_url)
            .map_err(|e| anyhow!("Invalid upload URL: {}, {}", raw_upload_url, e))?;
        let skip_upload = args.skip_upload || env::var("CODSPEED_SKIP_UPLOAD") == Ok("true".into());
        let token = args.token.or_else(|| env::var("CODSPEED_TOKEN").ok());
        Ok(Self {
            upload_url,
            token,
            working_directory: args.working_directory,
            command: args.command.join(" "),
            skip_upload,
            skip_setup: args.skip_setup,
        })
    }
}
