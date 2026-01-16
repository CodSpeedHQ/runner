use crate::api_client::CodSpeedAPIClient;
use crate::binary_installer::ensure_binary_installed;
use crate::config::CodSpeedConfig;
use crate::executor;
use crate::prelude::*;
use crate::project_config::ProjectConfig;
use crate::project_config::merger::ConfigMerger;
use crate::run::uploader::UploadResult;
use clap::Args;
use exec_harness::exec_targets::{ExecTarget, ExecTargetsFile, WalltimeExecutionOptions};
use exec_harness::walltime::WalltimeExecutionArgs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tempfile::NamedTempFile;

mod poll_results;

/// We temporarily force this name for all exec runs
pub const DEFAULT_REPOSITORY_NAME: &str = "local-runs";

const EXEC_HARNESS_COMMAND: &str = "exec-harness";
const EXEC_HARNESS_VERSION: &str = "1.0.0";

/// Wraps a command with exec-harness and the given walltime arguments.
///
/// This produces a shell command string like:
/// `exec-harness --warmup-time 1s --max-rounds 10 sleep 0.1`
pub fn wrap_with_exec_harness(
    walltime_args: &exec_harness::walltime::WalltimeExecutionArgs,
    command: &[String],
) -> String {
    shell_words::join(
        std::iter::once(EXEC_HARNESS_COMMAND)
            .chain(walltime_args.to_cli_args().iter().map(|s| s.as_str()))
            .chain(command.iter().map(|s| s.as_str())),
    )
}

/// Wraps exec-harness with a targets file path (no command on CLI).
///
/// This produces a shell command string like:
/// `exec-harness`
///
/// The targets file path is passed via CODSPEED_TARGETS_FILE env var.
pub fn wrap_with_exec_harness_targets_mode() -> String {
    EXEC_HARNESS_COMMAND.to_string()
}

/// Parse a duration string into nanoseconds
/// Supports humantime format: "1s", "500ms", "1.5s", "2m", "1h", etc.
/// Also supports pure numbers interpreted as seconds (e.g., "2" = 2s, "1.5" = 1.5s)
fn parse_duration_to_ns(s: &str) -> Result<u64> {
    let s = s.trim();

    // Try parsing as pure number first (interpret as seconds)
    if let Ok(seconds) = s.parse::<f64>() {
        return Ok((seconds * 1_000_000_000.0) as u64);
    }

    // Try parsing with humantime
    let duration: Duration = humantime::parse_duration(s)
        .with_context(|| format!("Invalid duration format: '{s}'. Expected format like '1s', '500ms', '2m', '1h' or a number in seconds"))?;

    Ok(duration.as_nanos() as u64)
}

/// Convert WalltimeExecutionArgs to WalltimeExecutionOptions (serializable format)
fn walltime_args_to_options(args: &WalltimeExecutionArgs) -> Result<WalltimeExecutionOptions> {
    Ok(WalltimeExecutionOptions {
        warmup_time_ns: args
            .warmup_time
            .as_ref()
            .map(|s| parse_duration_to_ns(s))
            .transpose()
            .context("Invalid warmup_time")?,
        max_time_ns: args
            .max_time
            .as_ref()
            .map(|s| parse_duration_to_ns(s))
            .transpose()
            .context("Invalid max_time")?,
        min_time_ns: args
            .min_time
            .as_ref()
            .map(|s| parse_duration_to_ns(s))
            .transpose()
            .context("Invalid min_time")?,
        max_rounds: args.max_rounds,
        min_rounds: args.min_rounds,
    })
}

/// Resolve targets from project config, merging options with CLI args
///
/// Returns a list of resolved targets with merged walltime options.
fn resolve_targets(
    project_config: &ProjectConfig,
    cli_walltime_args: &WalltimeExecutionArgs,
) -> Result<Vec<ExecTarget>> {
    let targets = project_config
        .targets
        .as_ref()
        .context("No targets defined in config")?;

    let global_walltime = project_config
        .options
        .as_ref()
        .and_then(|o| o.walltime.as_ref());

    let mut resolved = Vec::with_capacity(targets.len());

    for target in targets {
        // Merge walltime options: CLI > target > global
        let merged_walltime_args = ConfigMerger::merge_walltime_with_target(
            cli_walltime_args,
            target.options.as_ref(),
            global_walltime,
        );

        let walltime_options = walltime_args_to_options(&merged_walltime_args)?;

        resolved.push(ExecTarget {
            name: target.name.clone(),
            command: target.exec.clone(),
            walltime_options,
        });
    }

    Ok(resolved)
}

/// Write targets to a temporary JSON file for exec-harness
///
/// Returns the temp file handle (must be kept alive until exec-harness reads it)
fn write_targets_file(targets: Vec<ExecTarget>) -> Result<NamedTempFile> {
    let targets_file = ExecTargetsFile { targets };

    let mut temp_file = NamedTempFile::new().context("Failed to create temp file for targets")?;

    serde_json::to_writer(&mut temp_file, &targets_file)
        .context("Failed to serialize targets to JSON")?;

    temp_file.flush().context("Failed to flush targets file")?;

    Ok(temp_file)
}

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

/// Determines if we should use targets mode (multi-target from config)
fn should_use_targets_mode(args: &ExecArgs, project_config: Option<&ProjectConfig>) -> bool {
    // Use targets mode if:
    // 1. No CLI command provided
    // 2. Project config exists and has targets
    args.command.is_empty()
        && project_config
            .and_then(|c| c.targets.as_ref())
            .is_some_and(|t| !t.is_empty())
}

pub async fn run(
    args: ExecArgs,
    api_client: &CodSpeedAPIClient,
    codspeed_config: &CodSpeedConfig,
    project_config: Option<&ProjectConfig>,
    setup_cache_dir: Option<&Path>,
) -> Result<()> {
    // Determine execution mode: single command vs multi-target
    let use_targets_mode = should_use_targets_mode(&args, project_config);

    // Hold the temp file handle to keep it alive during execution
    let _targets_file_handle: Option<NamedTempFile>;

    let config = if use_targets_mode {
        let project_config = project_config.context(
            "No command provided and no project config found. \
            Either provide a command or create a codspeed.yaml with targets.",
        )?;

        info!(
            "Running in multi-target mode with {} targets from config",
            project_config.targets.as_ref().map_or(0, |t| t.len())
        );

        // Resolve targets from config with merged options
        let targets = resolve_targets(project_config, &args.walltime_args)?;

        // Write targets to temp file
        let temp_file = write_targets_file(targets)?;
        let targets_file_path = temp_file.path().to_path_buf();
        _targets_file_handle = Some(temp_file);

        debug!("Wrote targets file to: {}", targets_file_path.display());

        // Merge shared args only (walltime args are per-target in the file)
        let merged_shared =
            ConfigMerger::merge_shared_args(&args.shared, project_config.options.as_ref());

        // Create config with targets mode
        let raw_upload_url = merged_shared
            .upload_url
            .clone()
            .unwrap_or_else(|| "https://api.codspeed.io/upload".into());
        let upload_url = url::Url::parse(&raw_upload_url)
            .map_err(|e| anyhow!("Invalid upload URL: {raw_upload_url}, {e}"))?;

        crate::executor::Config {
            upload_url,
            token: merged_shared.token,
            repository_override: merged_shared
                .repository
                .map(|repo| {
                    crate::executor::config::RepositoryOverride::from_arg(
                        repo,
                        merged_shared.provider,
                    )
                })
                .transpose()?,
            working_directory: merged_shared.working_directory,
            mode: merged_shared.mode,
            instruments: crate::instruments::Instruments { mongodb: None },
            perf_unwinding_mode: merged_shared.perf_run_args.perf_unwinding_mode,
            enable_perf: merged_shared.perf_run_args.enable_perf,
            command: wrap_with_exec_harness_targets_mode(),
            profile_folder: merged_shared.profile_folder,
            skip_upload: merged_shared.skip_upload,
            skip_run: merged_shared.skip_run,
            skip_setup: merged_shared.skip_setup,
            allow_empty: merged_shared.allow_empty,
            targets_file_path: Some(targets_file_path),
        }
    } else {
        // Single command mode (backward compatible)
        if args.command.is_empty() {
            bail!(
                "No command provided. Either provide a command or create a codspeed.yaml with targets.\n\
                Example: codspeed exec -- ./my-benchmark\n\
                Or define targets in codspeed.yaml:\n\
                targets:\n\
                  - name: my-benchmark\n\
                    exec: [\"./my-benchmark\"]"
            );
        }

        _targets_file_handle = None;
        let merged_args = args.merge_with_project_config(project_config);
        crate::executor::Config::try_from(merged_args)?
    };

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
