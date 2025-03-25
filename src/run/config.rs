use crate::prelude::*;
use crate::run::instruments::Instruments;
use url::Url;

use crate::run::run_environment::RepositoryProvider;
use crate::run::RunArgs;

use super::RunnerMode;

#[derive(Debug)]
pub struct Config {
    pub upload_url: Url,
    pub token: Option<String>,
    pub repository_override: Option<RepositoryOverride>,
    pub working_directory: Option<String>,
    pub command: String,

    pub mode: RunnerMode,
    pub instruments: Instruments,

    pub skip_upload: bool,
    pub skip_setup: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RepositoryOverride {
    pub owner: String,
    pub repository: String,
    pub repository_provider: RepositoryProvider,
}

impl Config {
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }
}

#[cfg(test)]
impl Config {
    /// Constructs a new `Config` with default values for testing purposes
    pub fn test() -> Self {
        Self {
            upload_url: Url::parse(DEFAULT_UPLOAD_URL).unwrap(),
            token: None,
            repository_override: None,
            working_directory: None,
            command: "".into(),
            mode: RunnerMode::Instrumentation,
            instruments: Instruments::test(),
            skip_upload: false,
            skip_setup: false,
        }
    }
}

const DEFAULT_UPLOAD_URL: &str = "https://api.codspeed.io/upload";

impl TryFrom<RunArgs> for Config {
    type Error = Error;
    fn try_from(args: RunArgs) -> Result<Self> {
        let instruments = Instruments::try_from(&args)?;
        let raw_upload_url = args.upload_url.unwrap_or_else(|| DEFAULT_UPLOAD_URL.into());
        let upload_url = Url::parse(&raw_upload_url)
            .map_err(|e| anyhow!("Invalid upload URL: {}, {}", raw_upload_url, e))?;

        Ok(Self {
            upload_url,
            token: args.token,
            repository_override: args
                .repository
                .map(|respository_and_owner| -> Result<RepositoryOverride> {
                    let (owner, repository) =
                        extract_owner_and_repository_from_arg(&respository_and_owner)?;
                    Ok(RepositoryOverride {
                        owner,
                        repository,
                        repository_provider: args.provider.unwrap_or_default(),
                    })
                })
                .transpose()?,
            working_directory: args.working_directory,
            mode: args.mode,
            instruments,
            command: args.command.join(" "),
            skip_upload: args.skip_upload,
            skip_setup: args.skip_setup,
        })
    }
}

fn extract_owner_and_repository_from_arg(owner_and_repository: &str) -> Result<(String, String)> {
    let (owner, repository) = owner_and_repository
        .split_once('/')
        .context("Invalid owner/repository format")?;
    Ok((owner.to_string(), repository.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::run::instruments::MongoDBConfig;

    use super::*;

    #[test]
    fn test_try_from_env_empty() {
        let config = Config::try_from(RunArgs {
            upload_url: None,
            token: None,
            repository: None,
            provider: None,
            working_directory: None,
            mode: RunnerMode::Instrumentation,
            instruments: vec![],
            mongo_uri_env_name: None,
            skip_upload: false,
            skip_setup: false,
            command: vec!["cargo".into(), "codspeed".into(), "bench".into()],
        })
        .unwrap();
        assert_eq!(config.upload_url, Url::parse(DEFAULT_UPLOAD_URL).unwrap());
        assert_eq!(config.token, None);
        assert_eq!(config.repository_override, None);
        assert_eq!(config.working_directory, None);
        assert_eq!(config.instruments, Instruments { mongodb: None });
        assert!(!config.skip_upload);
        assert!(!config.skip_setup);
        assert_eq!(config.command, "cargo codspeed bench");
    }

    #[test]
    fn test_try_from_args() {
        let config = Config::try_from(RunArgs {
            upload_url: Some("https://example.com/upload".into()),
            token: Some("token".into()),
            repository: Some("owner/repo".into()),
            provider: Some(RepositoryProvider::GitLab),
            working_directory: Some("/tmp".into()),
            mode: RunnerMode::Instrumentation,
            instruments: vec!["mongodb".into()],
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
        assert_eq!(
            config.repository_override,
            Some(RepositoryOverride {
                owner: "owner".into(),
                repository: "repo".into(),
                repository_provider: RepositoryProvider::GitLab,
            })
        );
        assert_eq!(config.working_directory, Some("/tmp".into()));
        assert_eq!(
            config.instruments,
            Instruments {
                mongodb: Some(MongoDBConfig {
                    uri_env_name: Some("MONGODB_URI".into())
                })
            }
        );
        assert!(config.skip_upload);
        assert!(config.skip_setup);
        assert_eq!(config.command, "cargo codspeed bench");
    }

    #[test]
    fn test_extract_owner_and_repository_from_arg() {
        let owner_and_repository = "CodSpeedHQ/runner";
        let (owner, repository) =
            extract_owner_and_repository_from_arg(owner_and_repository).unwrap();
        assert_eq!(owner, "CodSpeedHQ");
        assert_eq!(repository, "runner");

        let owner_and_repository = "CodSpeedHQ_runner";

        let result = extract_owner_and_repository_from_arg(owner_and_repository);
        assert!(result.is_err());
    }
}
