use crate::api_client::CodSpeedAPIClient;
use crate::binary_installer::ensure_binary_installed;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::prelude::*;
use crate::run::uploader::UploadResult;
use clap::Args;
use std::path::Path;

mod poll_results;

/// We temporarily force this name for all exec runs
pub const DEFAULT_REPOSITORY_NAME: &str = "local-runs";

pub const EXEC_HARNESS_COMMAND: &str = "exec-harness";
const EXEC_HARNESS_VERSION: &str = "1.0.0";

#[derive(Args, Debug)]
pub struct ExecArgs {
    #[command(flatten)]
    pub shared: crate::run::ExecAndRunSharedArgs,

    /// Optional benchmark name (defaults to command filename)
    #[arg(long)]
    pub name: Option<String>,

    /// The command to execute with the exec harness
    pub command: Vec<String>,
}

pub async fn run(
    args: ExecArgs,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    let config = crate::executor::Config::try_from(args)?;
    let mut execution_context = executor::ExecutionContext::try_from((config, codspeed_config))?;
    debug!("config: {:#?}", execution_context.config);
    let executor = executor::get_executor_from_mode(
        &execution_context.config.mode,
        executor::ExecutorCommand::Exec,
    );

    let get_exec_harness_installer_url = || {
        format!(
            "https://github.com/CodSpeedHQ/runner/releases/download/exec-harness-v{EXEC_HARNESS_VERSION}/exec-harness-installer.sh"
        )
    };

    // Ensure the exec-harness is installed
    ensure_binary_installed(
        EXEC_HARNESS_COMMAND,
        EXEC_HARNESS_VERSION,
        get_exec_harness_installer_url,
    )
    .await?;

    let poll_results_fn = async |upload_result: &UploadResult| {
        poll_results::poll_results(api_client, upload_result).await
    };

    executor::execute_benchmarks(
        executor.as_ref(),
        &mut execution_context,
        setup_cache_dir,
        poll_results_fn,
        api_client,
    )
    .await?;

    Ok(())
}
