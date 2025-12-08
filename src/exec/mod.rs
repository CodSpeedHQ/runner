use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
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
    use crate::run_environment::RunEnvironment;

    // Convert ExecArgs to executor::Config
    let config = crate::executor::Config::try_from(args)?;

    // Create execution context
    let mut execution_context =
        crate::executor::ExecutionContext::try_from((config, codspeed_config))?;

    // Execute benchmarks
    crate::executor::execute_benchmarks(&mut execution_context, setup_cache_dir).await?;

    // Handle upload and polling
    if !execution_context.config.skip_upload {
        if execution_context.provider.get_run_environment() != RunEnvironment::Local {
            // If relevant, set the OIDC token for authentication
            // Note: OIDC tokens can expire quickly, so we set it just before the upload
            execution_context
                .provider
                .set_oidc_token(&mut execution_context.config)
                .await?;
        }

        start_group!("Uploading performance data");
        let executor = crate::executor::get_executor_from_mode(&execution_context.config.mode);
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
