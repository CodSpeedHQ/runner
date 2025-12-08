use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::prelude::*;
use clap::Args;
use std::path::Path;

mod poll_results;

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
    // Convert ExecArgs to executor::Config
    let config = executor::Config::try_from(args)?;

    // Create execution context
    let mut execution_context = executor::ExecutionContext::try_from((config, codspeed_config))?;

    // Execute benchmarks
    let executor = executor::get_executor_from_mode(
        &execution_context.config.mode,
        executor::ExecutorCommand::Exec,
    );
    executor::execute_benchmarks(executor.as_ref(), &mut execution_context, setup_cache_dir)
        .await?;

    // Handle upload and polling
    if !execution_context.config.skip_upload {
        start_group!("Uploading performance data");
        let upload_result =
            crate::run::uploader::upload(&execution_context, executor.name()).await?;
        end_group!();

        if execution_context.is_local() {
            poll_results::poll_results(api_client, upload_result.run_id).await?;
            end_group!();
        }
    }

    Ok(())
}
