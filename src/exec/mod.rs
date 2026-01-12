use crate::api_client::CodSpeedAPIClient;
use crate::binary_installer::ensure_binary_installed;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::prelude::*;
use crate::project_config::ProjectConfig;
use crate::project_config::merger::ConfigMerger;
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

    #[command(flatten)]
    pub walltime_args: exec_harness::walltime::WalltimeExecutionArgs,

    /// Optional benchmark name (defaults to command filename)
    #[arg(long)]
    pub name: Option<String>,

    /// The command to execute with the exec harness
    pub command: Vec<String>,
}

impl ExecArgs {
    /// Merge CLI args with project config if available
    ///
    /// CLI arguments take precedence over config values.
    pub fn merge_with_project_config(mut self, project_config: Option<&ProjectConfig>) -> Self {
        if let Some(project_config) = project_config {
            // Merge shared args
            self.shared =
                ConfigMerger::merge_shared_args(&self.shared, project_config.options.as_ref());
            // Merge walltime args
            self.walltime_args = ConfigMerger::merge_walltime_options(
                &self.walltime_args,
                project_config
                    .options
                    .as_ref()
                    .and_then(|o| o.walltime.as_ref()),
            );
        }
        self
    }
}

pub async fn run(
    args: ExecArgs,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
    project_config: Option<&ProjectConfig>,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    let merged_args = args.merge_with_project_config(project_config);

    let config = crate::executor::Config::try_from(merged_args)?;
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
