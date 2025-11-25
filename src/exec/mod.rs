use crate::api_client::CodSpeedAPIClient;
use crate::config::CodSpeedConfig;
use crate::prelude::*;
use clap::Args;
use std::path::Path;

mod run_with_harness;
use run_with_harness::wrap_command_with_harness;

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
    // Wrap the user's command with exec-harness BEFORE creating config
    let wrapped_command = wrap_command_with_harness(&args.command, args.name.as_deref())?;

    info!("Executing: {}", wrapped_command.join(" "));

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
        command: wrapped_command.join(" "), // Use wrapped command
        mode: args.shared.mode,
        instruments: crate::instruments::Instruments { mongodb: None }, // exec doesn't support MongoDB
        enable_perf: args.shared.perf_run_args.enable_perf,
        perf_unwinding_mode: args.shared.perf_run_args.perf_unwinding_mode,
        profile_folder: args.shared.profile_folder,
        skip_upload: args.shared.skip_upload,
        skip_run: args.shared.skip_run,
        skip_setup: args.shared.skip_setup,
        allow_empty: args.shared.allow_empty,
    };

    // Delegate to shared execution logic
    crate::executor::execute_benchmarks(config, api_client, codspeed_config, setup_cache_dir, false)
        .await
}
