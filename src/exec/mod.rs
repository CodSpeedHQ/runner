use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::{executor, prelude::*};
use clap::Args;
use runner_shared::walltime_results::{WalltimeBenchmark, WalltimeResults};
use std::path::Path;
use std::time::Instant;

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

    let mut context = executor::initialize_execution_environment(config, codspeed_config).await?;

    let executor = executor::get_executor_from_mode(&context.config.mode, true);

    if !context.config.skip_setup {
        start_group!("Preparing the environment");

        executor
            .setup(&context.system_info, setup_cache_dir)
            .await?;

        info!("Environment ready");
        end_group!();
    }

    if !context.config.skip_run {
        start_opened_group!("Running the benchmarks");

        let start_time = Instant::now();
        executor
            .run(
                &context.config,
                &context.system_info,
                &context.run_data,
                &None, // TODO: Do we support mongo tracer in exec ?
            )
            .await?;

        let duration = start_time.elapsed();

        dbg!(duration);

        // TODO: use args.name or derive from command
        let bench_name = "bench_name".to_string();
        let walltime_benchmark = WalltimeBenchmark::from_runtime_data(
            bench_name.clone(),
            format!("standalone_run::{bench_name}"),
            vec![1],
            vec![duration.as_nanos()],
            Some(duration.as_nanos()),
        );

        let walltime_results = WalltimeResults::from_benchmarks(vec![walltime_benchmark])
            .expect("Failed to create walltime results");

        walltime_results
            .save_to_file(&context.run_data.profile_folder)
            .expect("Failed to save walltime results");

        executor
            .teardown(&context.config, &context.system_info, &context.run_data)
            .await?;

        context
            .logger
            .persist_log_to_profile_folder(&context.run_data)?;

        end_group!();
    } else {
        debug!("Skipping the run of the benchmarks");
    }

    if !context.config.skip_upload {
        executor::upload_and_poll_results(executor.as_ref(), &mut context, api_client, false)
            .await?;
    }

    Ok(())
}
