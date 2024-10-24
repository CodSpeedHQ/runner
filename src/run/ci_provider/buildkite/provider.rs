use std::env;

use lazy_static::lazy_static;
use regex::Regex;
use simplelog::SharedLogger;

use crate::prelude::*;
use crate::run::{
    ci_provider::{
        interfaces::{CIProviderMetadata, RepositoryProvider, RunEvent},
        provider::{CIProvider, CIProviderDetector},
    },
    config::Config,
    helpers::{find_repository_root, get_env_variable},
};

use super::logger::BuildkiteLogger;

#[derive(Debug)]
pub struct BuildkiteProvider {
    owner: String,
    repository: String,
    ref_: String,
    head_ref: Option<String>,
    base_ref: Option<String>,
    event: RunEvent,
    repository_root_path: String,
}

lazy_static! {
    static ref GITHUB_URL_REGEX: Regex = Regex::new(
        r"(?x)
            (?:https://github.com/|git@github.com:)
            (?P<owner>[^/]+)/(?P<repository>[^/.]+)\.git
        "
    )
    .expect("Failed to compile GitHub URL regex");
}

pub fn get_pr_number() -> Result<Option<u64>> {
    Ok(get_env_variable("BUILDKITE_PULL_REQUEST")?.parse().ok())
}

pub fn get_run_event() -> Result<RunEvent> {
    let is_pr = get_pr_number()?.is_some();

    if is_pr {
        Ok(RunEvent::PullRequest)
    } else {
        Ok(RunEvent::Push)
    }
}

pub fn get_ref() -> Result<String> {
    let pr_number = get_pr_number()?;

    if let Some(pr_number) = pr_number {
        Ok(format!("refs/pull/{}/merge", pr_number))
    } else {
        Ok(format!(
            "refs/heads/{}",
            get_env_variable("BUILDKITE_BRANCH")?
        ))
    }
}

pub fn get_owner_and_repository() -> Result<(String, String)> {
    let repository_url = get_env_variable("BUILDKITE_REPO")?;
    let captures = GITHUB_URL_REGEX
        .captures(&repository_url)
        .context("Failed to parse the GitHub repository URL")?;

    let owner = captures
        .name("owner")
        .context("Failed to parse the GitHub repository URL")?
        .as_str();
    let repository = captures
        .name("repository")
        .context("Failed to parse the GitHub repository URL")?
        .as_str();

    Ok((owner.into(), repository.into()))
}

impl TryFrom<&Config> for BuildkiteProvider {
    type Error = Error;
    fn try_from(config: &Config) -> Result<Self> {
        if config.token.is_none() {
            bail!("Token authentication is required for Buildkite");
        }

        let is_pr = get_pr_number()?.is_some();
        let (owner, repository) = get_owner_and_repository()?;

        let repository_root_path = match find_repository_root(&std::env::current_dir()?) {
            Some(mut path) => {
                // Add a trailing slash to the path
                path.push("");
                path.to_string_lossy().to_string()
            }
            None => format!(
                "/buildkite/builds/{}/{}/{}/",
                get_env_variable("BUILDKITE_AGENT_NAME")?,
                get_env_variable("BUILDKITE_ORGANIZATION_SLUG")?,
                get_env_variable("BUILDKITE_PIPELINE_SLUG")?,
            ),
        };

        Ok(Self {
            owner: owner.clone(),
            repository: repository.clone(),
            ref_: get_ref()?,
            base_ref: if is_pr {
                Some(get_env_variable("BUILDKITE_PULL_REQUEST_BASE_BRANCH")?)
            } else {
                None
            },
            head_ref: if is_pr {
                Some(get_env_variable("BUILDKITE_BRANCH")?)
            } else {
                None
            },
            event: get_run_event()?,
            repository_root_path,
        })
    }
}

impl CIProviderDetector for BuildkiteProvider {
    fn detect() -> bool {
        env::var("BUILDKITE") == Ok("true".into())
    }
}

impl CIProvider for BuildkiteProvider {
    fn get_repository_provider(&self) -> RepositoryProvider {
        RepositoryProvider::GitHub
    }

    fn get_logger(&self) -> Box<dyn SharedLogger> {
        Box::new(BuildkiteLogger::new())
    }

    fn get_provider_name(&self) -> &'static str {
        "Buildkite"
    }

    fn get_provider_slug(&self) -> &'static str {
        "buildkite"
    }

    fn get_ci_provider_metadata(&self) -> Result<CIProviderMetadata> {
        Ok(CIProviderMetadata {
            base_ref: self.base_ref.clone(),
            head_ref: self.head_ref.clone(),
            event: self.event.clone(),
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            ref_: self.ref_.clone(),
            repository_root_path: self.repository_root_path.clone(),
            gh_data: None,
            gl_data: None,
            sender: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;
    use temp_env::{with_var, with_vars};

    use crate::VERSION;

    use super::*;

    #[test]
    fn test_detect() {
        with_var("BUILDKITE", Some("true"), || {
            assert!(BuildkiteProvider::detect());
        });
    }

    #[test]
    fn test_get_owner_and_repository() {
        with_var(
            "BUILDKITE_REPO",
            Some("https://github.com/my-org/adrien-python-test.git"),
            || {
                let (owner, repository) = get_owner_and_repository().unwrap();
                assert_eq!(owner, "my-org");
                assert_eq!(repository, "adrien-python-test");
            },
        );

        with_var(
            "BUILDKITE_REPO",
            Some("git@github.com:my-org/adrien-python-test.git"),
            || {
                let (owner, repository) = get_owner_and_repository().unwrap();
                assert_eq!(owner, "my-org");
                assert_eq!(repository, "adrien-python-test");
            },
        );
    }

    #[test]
    fn test_try_from_push_main() {
        with_vars(
            [
                ("BUILDKITE_AGENT_NAME", Some("7b10eca7600b-1")),
                ("BUILDKITE_BRANCH", Some("main")),
                ("BUILDKITE_BUILD_NUMBER", Some("1")),
                ("BUILDKITE_COMMIT", Some("abc123")),
                ("BUILDKITE_ORGANIZATION_SLUG", Some("my-org")),
                ("BUILDKITE_PIPELINE_SLUG", Some("buildkite-test")),
                ("BUILDKITE_PULL_REQUEST_BASE_BRANCH", Some("")),
                ("BUILDKITE_PULL_REQUEST", Some("")),
                (
                    "BUILDKITE_REPO",
                    Some("https://github.com/my-org/adrien-python-test.git"),
                ),
                ("BUILDKITE", Some("true")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let provider = BuildkiteProvider::try_from(&config).unwrap();

                assert_eq!(provider.owner, "my-org");
                assert_eq!(provider.repository, "adrien-python-test");
                assert_eq!(provider.ref_, "refs/heads/main");
                assert_eq!(provider.base_ref, None);
                assert_eq!(provider.head_ref, None);
                assert_eq!(provider.event, RunEvent::Push);
                assert_eq!(
                    provider.repository_root_path,
                    "/buildkite/builds/7b10eca7600b-1/my-org/buildkite-test/"
                );
            },
        );
    }

    #[test]
    fn test_try_from_pull_request() {
        with_vars(
            [
                ("BUILDKITE_AGENT_NAME", Some("7b10eca7600b-1")),
                ("BUILDKITE_BRANCH", Some("feat/codspeed-runner")),
                ("BUILDKITE_BUILD_NUMBER", Some("1")),
                ("BUILDKITE_COMMIT", Some("abc123")),
                ("BUILDKITE_ORGANIZATION_SLUG", Some("my-org")),
                ("BUILDKITE_PIPELINE_SLUG", Some("buildkite-test")),
                ("BUILDKITE_PULL_REQUEST_BASE_BRANCH", Some("main")),
                ("BUILDKITE_PULL_REQUEST", Some("22")),
                (
                    "BUILDKITE_REPO",
                    Some("git@github.com:my-org/adrien-python-test.git"),
                ),
                ("BUILDKITE", Some("true")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let provider = BuildkiteProvider::try_from(&config).unwrap();

                assert_eq!(provider.owner, "my-org");
                assert_eq!(provider.repository, "adrien-python-test");
                assert_eq!(provider.ref_, "refs/pull/22/merge");
                assert_eq!(provider.base_ref, Some("main".into()));
                assert_eq!(provider.head_ref, Some("feat/codspeed-runner".into()));
                assert_eq!(provider.event, RunEvent::PullRequest);
                assert_eq!(
                    provider.repository_root_path,
                    "/buildkite/builds/7b10eca7600b-1/my-org/buildkite-test/"
                );
            },
        );
    }

    #[test]
    fn test_pull_request_provider_metadata() {
        with_vars(
            [
                ("BUILDKITE_AGENT_NAME", Some("7b10eca7600b-1")),
                ("BUILDKITE_BRANCH", Some("feat/codspeed-runner")),
                ("BUILDKITE_BUILD_NUMBER", Some("1")),
                ("BUILDKITE_COMMIT", Some("abc123")),
                ("BUILDKITE_ORGANIZATION_SLUG", Some("my-org")),
                ("BUILDKITE_PIPELINE_SLUG", Some("buildkite-test")),
                ("BUILDKITE_PULL_REQUEST_BASE_BRANCH", Some("main")),
                ("BUILDKITE_PULL_REQUEST", Some("22")),
                (
                    "BUILDKITE_REPO",
                    Some("git@github.com:my-org/adrien-python-test.git"),
                ),
                ("BUILDKITE", Some("true")),
            ],
            || {
                let config = Config {
                    token: Some("token".into()),
                    ..Config::test()
                };
                let provider = BuildkiteProvider::try_from(&config).unwrap();
                let provider_metadata = provider.get_ci_provider_metadata().unwrap();

                assert_json_snapshot!(provider_metadata, {
                    ".runner.version" => insta::dynamic_redaction(|value,_path| {
                        assert_eq!(value.as_str().unwrap(), VERSION.to_string());
                        "[version]"
                    }),
                })
            },
        );
    }
}
