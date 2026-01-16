use clap::Parser;
use exec_harness::MeasurementMode;
use exec_harness::analysis;
use exec_harness::exec_targets::ExecTargetsFile;
use exec_harness::prelude::*;
use exec_harness::uri;
use exec_harness::walltime;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "exec-harness")]
#[command(
    version,
    about = "CodSpeed exec harness - wraps commands with performance instrumentation"
)]
struct Args {
    /// Optional benchmark name, else the command will be used as the name
    #[arg(long)]
    name: Option<String>,

    /// Set by the runner, should be coherent with the executor being used
    #[arg(short, long, global = true, env = "CODSPEED_RUNNER_MODE", hide = true)]
    measurement_mode: Option<MeasurementMode>,

    #[command(flatten)]
    execution_args: walltime::WalltimeExecutionArgs,

    /// The command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

/// Load targets from a JSON file
fn load_targets_file(path: &Path) -> Result<ExecTargetsFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read targets file: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse targets file: {}", path.display()))
}

/// Run in multi-target mode with targets from file
fn run_multi_target(targets_file: ExecTargetsFile, measurement_mode: Option<MeasurementMode>) -> Result<()> {
    info!("Running {} targets from config", targets_file.targets.len());

    match measurement_mode {
        Some(MeasurementMode::Walltime) | None => {
            walltime::perform_targets(targets_file.targets)?;
        }
        Some(MeasurementMode::Memory) => {
            analysis::perform_targets(targets_file.targets)?;
        }
        Some(MeasurementMode::Simulation) => {
            bail!("Simulation measurement mode is not yet supported by exec-harness");
        }
    }

    Ok(())
}

/// Run in single command mode (backward compatible)
fn run_single_command(args: Args) -> Result<()> {
    if args.command.is_empty() {
        bail!("Error: No command provided");
    }

    let bench_name_and_uri = uri::generate_name_and_uri(&args.name, &args.command);

    match args.measurement_mode {
        Some(MeasurementMode::Walltime) | None => {
            let execution_options: walltime::ExecutionOptions = args.execution_args.try_into()?;

            walltime::perform(bench_name_and_uri, args.command, &execution_options)?;
        }
        Some(MeasurementMode::Memory) => {
            analysis::perform(bench_name_and_uri, args.command)?;
        }
        Some(MeasurementMode::Simulation) => {
            bail!("Simulation measurement mode is not yet supported by exec-harness");
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    env_logger::builder()
        .parse_env(env_logger::Env::new().filter_or("CODSPEED_LOG", "info"))
        .format_timestamp(None)
        .init();

    debug!("Starting exec-harness with pid {}", std::process::id());

    let args = Args::parse();

    // Check for multi-target mode via env var
    if let Ok(targets_file_path) = std::env::var("CODSPEED_TARGETS_FILE") {
        debug!("Running in multi-target mode with targets from: {targets_file_path}");
        let targets_file = load_targets_file(Path::new(&targets_file_path))?;
        run_multi_target(targets_file, args.measurement_mode)?;
    } else {
        // Single command mode (backward compatible)
        run_single_command(args)?;
    }

    Ok(())
}
