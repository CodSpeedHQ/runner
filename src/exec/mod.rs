use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::prelude::*;
use clap::Args;
use std::path::Path;

mod poll_results;

/// We temporarily force this name for all exec runs
pub const DEFAULT_REPOSITORY_NAME: &str = "local-runs";

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
    let executor = executor::get_executor_from_mode(
        &execution_context.config.mode,
        executor::ExecutorCommand::Exec,
    );

    let poll_results_fn = |run_id: String| poll_results::poll_results(api_client, run_id);

    executor::execute_benchmarks(
        executor.as_ref(),
        &mut execution_context,
        setup_cache_dir,
        poll_results_fn,
    )
    .await?;

    Ok(())
}
