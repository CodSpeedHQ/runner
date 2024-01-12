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

    pub mongodb: bool,
    pub mongo_uri_env_name: Option<String>,

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
            mongodb: false,
            mongo_uri_env_name: None,
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
        let mongo_uri_env_name = args
            .mongo_uri_env_name
            .or_else(|| env::var("CODSPEED_MONGO_INSTR_URI_ENV_NAME").ok());
        Ok(Self {
            upload_url,
            token,
            working_directory: args.working_directory,
            mongodb: args.mongo_db,
            mongo_uri_env_name,
            command: args.command.join(" "),
            skip_upload,
            skip_setup: args.skip_setup,
        })
    }
}

#[cfg(test)]
mod tests {
    use temp_env::{with_var, with_vars};

    use super::*;

    #[test]
    fn test_try_from_env_empty() {
        // TODO: this test fails if we remove the `with_var` call, open an issue on https://github.com/vmx/temp-env with a reproduction
        with_var("FOO", Some("bar"), || {
            let config = Config::try_from(AppArgs {
                upload_url: None,
                token: None,
                working_directory: None,
                mongo_db: false,
                mongo_uri_env_name: None,
                skip_upload: false,
                skip_setup: false,
                command: vec!["cargo".into(), "codspeed".into(), "bench".into()],
            })
            .unwrap();
            assert_eq!(config.upload_url, Url::parse(DEFAULT_UPLOAD_URL).unwrap());
            assert_eq!(config.token, None);
            assert_eq!(config.working_directory, None);
            assert!(!config.mongodb);
            assert_eq!(config.mongo_uri_env_name, None);
            assert!(!config.skip_upload);
            assert!(!config.skip_setup);
            assert_eq!(config.command, "cargo codspeed bench");
        });
    }

    #[test]
    fn test_try_from_args() {
        let config = Config::try_from(AppArgs {
            upload_url: Some("https://example.com/upload".into()),
            token: Some("token".into()),
            working_directory: Some("/tmp".into()),
            mongo_db: true,
            mongo_uri_env_name: Some("MONGODB_URI".into()),
            skip_upload: true,
            skip_setup: true,
            command: vec!["cargo".into(), "codspeed".into(), "bench".into()],
        })
        .unwrap();

        assert_eq!(
            config.upload_url,
            Url::parse("https://example.com/upload").unwrap()
        );
        assert_eq!(config.token, Some("token".into()));
        assert_eq!(config.working_directory, Some("/tmp".into()));
        assert!(config.mongodb);
        assert_eq!(config.mongo_uri_env_name, Some("MONGODB_URI".into()));
        assert!(config.skip_upload);
        assert!(config.skip_setup);
        assert_eq!(config.command, "cargo codspeed bench");
    }

    #[test]
    fn test_try_from_full() {
        with_vars(
            vec![
                ("CODSPEED_TOKEN", Some("token_from_env")),
                (
                    "CODSPEED_MONGO_INSTR_URI_ENV_NAME",
                    Some("MONGODB_URI_FROM_ENV"),
                ),
                ("CODSPEED_SKIP_UPLOAD", Some("true")),
            ],
            || {
                let config = Config::try_from(AppArgs {
                    upload_url: None,
                    token: None,
                    working_directory: None,
                    mongo_db: true,
                    mongo_uri_env_name: None,
                    skip_upload: false,
                    skip_setup: false,
                    command: vec!["cargo".into(), "codspeed".into(), "bench".into()],
                })
                .unwrap();

                assert_eq!(config.upload_url, Url::parse(DEFAULT_UPLOAD_URL).unwrap());
                assert_eq!(config.token, Some("token_from_env".into()));
                assert_eq!(config.working_directory, None);
                assert!(config.mongodb);
                assert_eq!(
                    config.mongo_uri_env_name,
                    Some("MONGODB_URI_FROM_ENV".into())
                );
                assert!(config.skip_upload);
                assert!(!config.skip_setup);
                assert_eq!(config.command, "cargo codspeed bench");
            },
        );
    }
}
