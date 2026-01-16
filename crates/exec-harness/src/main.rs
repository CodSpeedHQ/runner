use clap::Parser;
use exec_harness::MeasurementMode;
use exec_harness::analysis;
use exec_harness::prelude::*;
use exec_harness::uri;
use exec_harness::walltime;

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

fn main() -> Result<()> {
    env_logger::builder()
        .parse_env(env_logger::Env::new().filter_or("CODSPEED_LOG", "info"))
        .format_timestamp(None)
        .init();

    debug!("Starting exec-harness with pid {}", std::process::id());

    let args = Args::parse();

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
