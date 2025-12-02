use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::runner_mode::RunnerMode;
use crate::{executor, prelude::*};
use clap::Args;
use std::path::Path;

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
    if args.shared.mode != RunnerMode::Simulation {
        bail!("The 'exec' command only supports 'simulation' mode.");
    }

    // Convert ExecArgs to executor::Config using shared args
    let config = crate::executor::Config {
        upload_url: args
            .shared
            .upload_url
            .as_ref()
            .map(|url| url.parse())
            .transpose()
            .map_err(|e| anyhow!("Invalid upload URL: {e}"))?
            .unwrap_or_else(|| {
                "https://api.codspeed.io/upload"
                    .parse()
                    .expect("Default URL should be valid")
            }),
        token: args.shared.token,
        repository_override: args
            .shared
            .repository
            .map(|repo| {
                crate::executor::config::RepositoryOverride::from_arg(repo, args.shared.provider)
            })
            .transpose()?,
        working_directory: args.shared.working_directory,
        command: args.command.join(" "),
        mode: args.shared.mode,
        // TODO: Support mongo tracer in exec ?
        instruments: crate::instruments::Instruments { mongodb: None },
        enable_perf: args.shared.perf_run_args.enable_perf,
        perf_unwinding_mode: args.shared.perf_run_args.perf_unwinding_mode,
        profile_folder: args.shared.profile_folder,
        skip_upload: args.shared.skip_upload,
        skip_run: args.shared.skip_run,
        skip_setup: args.shared.skip_setup,
        allow_empty: args.shared.allow_empty,
    };

    let mut context = executor::initialize_execution_context(config, codspeed_config).await?;

    let executor = executor::get_executor_from_mode(&context.executor_config.mode, true);

    if !context.executor_config.skip_setup {
        start_group!("Preparing the environment");

        executor
            .setup(&context.system_info, setup_cache_dir)
            .await?;

        info!("Environment ready");
        end_group!();
    }

    if !context.executor_config.skip_run {
        start_opened_group!("Running the benchmarks");

        executor
            .run(
                &context.executor_config,
                &context.system_info,
                &context.run_data,
                &None, // TODO: Do we support mongo tracer in exec ?
            )
            .await?;

        executor
            .teardown(
                &context.executor_config,
                &context.system_info,
                &context.run_data,
            )
            .await?;

        context
            .logger
            .persist_log_to_profile_folder(&context.run_data)?;

        end_group!();
    } else {
        debug!("Skipping the run of the benchmarks");
    }

    if !context.executor_config.skip_upload {
        executor::upload_and_poll_results(executor.as_ref(), &mut context, api_client, false)
            .await?;
    }

    Ok(())
}
