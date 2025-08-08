use crate::VERSION;
use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::prelude::*;
use crate::run::{config::Config, logger::Logger};
use check_system::SystemInfo;
use clap::{Args, ValueEnum};
use instruments::mongo_tracer::{MongoTracer, install_mongodb_tracer};
use run_environment::interfaces::{RepositoryProvider, RunEnvironment};
use runner::get_run_data;
use serde::Serialize;
use std::path::PathBuf;

pub mod check_system;
pub mod helpers;
mod instruments;
mod poll_results;
pub mod run_environment;
pub mod runner;
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
  https://codspeed.io     /_/          runner v{VERSION}
"#
    );
    println!("{banner}");
    debug!("codspeed v{VERSION}");
}

#[derive(Debug, Copy, Clone, PartialEq, ValueEnum, Default)]
pub enum UnwindingMode {
    /// Use the frame pointer for unwinding. Requires the binary to be compiled with frame pointers enabled.
    #[clap(name = "fp")]
    FramePointer,

    /// Use DWARF unwinding. This does not require any special compilation flags and is enabled by default.
    #[default]
    Dwarf,
}

#[derive(Args, Debug, Clone)]
pub struct PerfRunArgs {
    /// Enable the linux perf profiler to collect granular performance data.
    /// This is only supported on Linux.
    #[arg(long, env = "CODSPEED_PERF_ENABLED", default_value_t = true)]
    enable_perf: bool,

    /// The unwinding mode that should be used with perf to collect the call stack.
    #[arg(long, env = "CODSPEED_PERF_UNWINDING_MODE")]
    perf_unwinding_mode: Option<UnwindingMode>,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The upload URL to use for uploading the results, useful for on-premises installations
    #[arg(long, env = "CODSPEED_UPLOAD_URL")]
    pub upload_url: Option<String>,

    /// The token to use for uploading the results,
    #[arg(long, env = "CODSPEED_TOKEN")]
    pub token: Option<String>,

    /// The repository the benchmark is associated with, under the format `owner/repo`.
    #[arg(short, long, env = "CODSPEED_REPOSITORY")]
    pub repository: Option<String>,

    /// The repository provider to use in case --repository is used. Defaults to github
    #[arg(
        long,
        env = "CODSPEED_PROVIDER",
        requires = "repository",
        ignore_case = true
    )]
    pub provider: Option<RepositoryProvider>,

    /// The directory where the command will be executed.
    #[arg(long)]
    pub working_directory: Option<String>,

    /// The mode to run the benchmarks in.
    #[arg(long, value_enum, env = "CODSPEED_RUNNER_MODE")]
    pub mode: RunnerMode,

    /// Comma-separated list of instruments to enable. Possible values: mongodb.
    #[arg(long, value_delimiter = ',')]
    pub instruments: Vec<String>,

    /// The name of the environment variable that contains the MongoDB URI to patch.
    /// If not provided, user will have to provide it dynamically through a CodSpeed integration.
    ///
    /// Only used if the `mongodb` instrument is enabled.
    #[arg(long)]
    pub mongo_uri_env_name: Option<String>,

    /// Profile folder to use for the run.
    #[arg(long)]
    pub profile_folder: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub message_format: Option<MessageFormat>,

    /// Only for debugging purposes, skips the upload of the results
    #[arg(
        long,
        default_value = "false",
        hide = true,
        env = "CODSPEED_SKIP_UPLOAD"
    )]
    pub skip_upload: bool,
    /// Used internally to upload the results after running the benchmarks in a sandbox environment
    /// with no internet access
    #[arg(long, default_value = "false", hide = true)]
    pub skip_run: bool,

    /// Only for debugging purposes, skips the setup of the runner
    #[arg(long, default_value = "false", hide = true)]
    pub skip_setup: bool,

    #[command(flatten)]
    pub perf_run_args: PerfRunArgs,

    /// The bench command to run
    pub command: Vec<String>,
}

#[derive(ValueEnum, Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunnerMode {
    Instrumentation,
    Walltime,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum MessageFormat {
    Json,
}

#[cfg(test)]
impl RunArgs {
    /// Constructs a new `RunArgs` with default values for testing purposes
    pub fn test() -> Self {
        Self {
            upload_url: None,
            token: None,
            repository: None,
            provider: None,
            working_directory: None,
            mode: RunnerMode::Instrumentation,
            instruments: vec![],
            mongo_uri_env_name: None,
            message_format: None,
            profile_folder: None,
            skip_upload: false,
            skip_run: false,
            skip_setup: false,
            perf_run_args: PerfRunArgs {
                enable_perf: false,
                perf_unwinding_mode: None,
            },
            command: vec![],
        }
    }
}

pub async fn run(
    args: RunArgs,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
) -> Result<()> {
    let output_json = args.message_format == Some(MessageFormat::Json);
    let mut config = Config::try_from(args)?;
    let provider = run_environment::get_provider(&config)?;
    let logger = Logger::new(&provider)?;

    if provider.get_run_environment() != RunEnvironment::Local {
        show_banner();
    }
    debug!("config: {config:#?}");

    if provider.get_run_environment() == RunEnvironment::Local {
        if codspeed_config.auth.token.is_none() {
            bail!("You have to authenticate the CLI first. Run `codspeed auth login`.");
        }
        debug!("Using the token from the CodSpeed configuration file");
        config.set_token(codspeed_config.auth.token.clone());
    }

    let system_info = SystemInfo::new()?;
    check_system::check_system(&system_info)?;

    let executor = runner::get_executor_from_mode(&config.mode);

    if !config.skip_setup {
        start_group!("Preparing the environment");
        executor.setup(&system_info).await?;
        // TODO: refactor and move directly in the Instruments struct as a `setup` method
        if config.instruments.is_mongodb_enabled() {
            install_mongodb_tracer().await?;
        }
        info!("Environment ready");
        end_group!();
    }

    let run_data = get_run_data(&config)?;

    if !config.skip_run {
        start_opened_group!("Running the benchmarks");

        // TODO: refactor and move directly in the Instruments struct as a `start` method
        let mongo_tracer = if let Some(mongodb_config) = &config.instruments.mongodb {
            let mut mongo_tracer = MongoTracer::try_from(&run_data.profile_folder, mongodb_config)?;
            mongo_tracer.start().await?;
            Some(mongo_tracer)
        } else {
            None
        };

        executor
            .run(&config, &system_info, &run_data, &mongo_tracer)
            .await?;

        // TODO: refactor and move directly in the Instruments struct as a `stop` method
        if let Some(mut mongo_tracer) = mongo_tracer {
            mongo_tracer.stop().await?;
        }
        executor.teardown(&config, &system_info, &run_data).await?;

        logger.persist_log_to_profile_folder(&run_data)?;

        end_group!();
    } else {
        debug!("Skipping the run of the benchmarks");
    };

    if !config.skip_upload {
        start_group!("Uploading performance data");
        let upload_result =
            uploader::upload(&config, &system_info, &provider, &run_data, executor.name()).await?;
        end_group!();

        if provider.get_run_environment() == RunEnvironment::Local {
            poll_results::poll_results(api_client, &provider, upload_result.run_id, output_json)
                .await?;
            end_group!();
        }
    }

    Ok(())
}

// We have to implement this manually, because deriving the trait makes the CLI values `git-hub`
// and `git-lab`
impl clap::ValueEnum for RepositoryProvider {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::GitLab, Self::GitHub]
    }
    fn to_possible_value<'a>(&self) -> ::std::option::Option<clap::builder::PossibleValue> {
        match self {
            Self::GitLab => Some(clap::builder::PossibleValue::new("gitlab").aliases(["gl"])),
            Self::GitHub => Some(clap::builder::PossibleValue::new("github").aliases(["gh"])),
        }
    }
}
