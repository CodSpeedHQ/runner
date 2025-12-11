use crate::VERSION;
use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::executor::Config;
use crate::prelude::*;
use crate::run_environment::interfaces::RepositoryProvider;
use crate::runner_mode::RunnerMode;
use clap::{Args, ValueEnum};
use std::path::Path;
use std::path::PathBuf;

pub mod check_system;
pub mod helpers;
pub(crate) mod poll_results;
pub(crate) mod uploader;

pub mod logger;

pub(crate) fn show_banner() {
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
    pub enable_perf: bool,

    /// The unwinding mode that should be used with perf to collect the call stack.
    #[arg(long, env = "CODSPEED_PERF_UNWINDING_MODE")]
    pub perf_unwinding_mode: Option<UnwindingMode>,
}

/// Arguments shared between run and exec commands
#[derive(Args, Debug, Clone)]
pub struct ExecAndRunSharedArgs {
    /// The upload URL to use for uploading the results, useful for on-premises installations
    #[arg(long, env = "CODSPEED_UPLOAD_URL")]
    pub upload_url: Option<String>,

    /// The token to use for uploading the results,
    ///
    /// It can be either a CodSpeed token retrieved from the repository setting
    /// or an OIDC token issued by the identity provider.
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
    #[arg(short, long, value_enum, env = "CODSPEED_RUNNER_MODE")]
    pub mode: RunnerMode,

    /// Profile folder to use for the run.
    #[arg(long)]
    pub profile_folder: Option<PathBuf>,

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

    /// Allow runs without any benchmarks to succeed instead of failing
    #[arg(long, default_value = "false", hide = true)]
    pub allow_empty: bool,

    #[command(flatten)]
    pub perf_run_args: PerfRunArgs,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub shared: ExecAndRunSharedArgs,

    /// Comma-separated list of instruments to enable. Possible values: mongodb.
    #[arg(long, value_delimiter = ',')]
    pub instruments: Vec<String>,

    /// The name of the environment variable that contains the MongoDB URI to patch.
    /// If not provided, user will have to provide it dynamically through a CodSpeed integration.
    ///
    /// Only used if the `mongodb` instrument is enabled.
    #[arg(long)]
    pub mongo_uri_env_name: Option<String>,

    #[arg(long, hide = true)]
    pub message_format: Option<MessageFormat>,

    /// The bench command to run
    pub command: Vec<String>,
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
            shared: ExecAndRunSharedArgs {
                upload_url: None,
                token: None,
                repository: None,
                provider: None,
                working_directory: None,
                mode: RunnerMode::Simulation,
                profile_folder: None,
                skip_upload: false,
                skip_run: false,
                skip_setup: false,
                allow_empty: false,
                perf_run_args: PerfRunArgs {
                    enable_perf: false,
                    perf_unwinding_mode: None,
                },
            },
            instruments: vec![],
            mongo_uri_env_name: None,
            message_format: None,
            command: vec![],
        }
    }
}

pub async fn run(
    args: RunArgs,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    let output_json = args.message_format == Some(MessageFormat::Json);
    let config = Config::try_from(args)?;

    // Create execution context
    let mut execution_context = executor::ExecutionContext::try_from((config, codspeed_config))?;

    if !execution_context.is_local() {
        show_banner();
    }
    debug!("config: {:#?}", execution_context.config);

    // Execute benchmarks
    let executor = executor::get_executor_from_mode(
        &execution_context.config.mode,
        executor::ExecutorCommand::Run,
    );

    let run_environment_metadata = execution_context.provider.get_run_environment_metadata()?;
    let poll_results_fn = |run_id: String| {
        poll_results::poll_results(api_client, &run_environment_metadata, run_id, output_json)
    };
    executor::execute_benchmarks(
        executor.as_ref(),
        &mut execution_context,
        setup_cache_dir,
        poll_results_fn,
    )
    .await?;

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
