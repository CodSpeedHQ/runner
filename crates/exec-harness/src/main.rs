use crate::prelude::*;
use crate::walltime::WalltimeResults;
use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;
use runner_shared::walltime_results::WalltimeBenchmark;
use std::path::PathBuf;

mod prelude;
mod uri;
mod walltime;

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

    let args = Args::parse();

    if args.command.is_empty() {
        bail!("Error: No command provided");
    }

    let uri::NameAndUri {
        name: bench_name,
        uri: bench_uri,
    } = uri::generate_name_and_uri(&args.name, &args.command);

    let hooks = InstrumentHooks::instance();

    // TODO(COD-1736): Stop impersonating codspeed-rust 🥸
    hooks
        .set_integration("codspeed-rust", env!("CARGO_PKG_VERSION"))
        .unwrap();

    // Build execution options from CLI args
    let execution_options: walltime::ExecutionOptions = args.execution_args.try_into()?;

    let times_per_round_ns =
        walltime::perform(bench_uri.clone(), args.command, &execution_options)?;

    // Collect walltime results
    let max_time_ns = times_per_round_ns.iter().copied().max();
    let walltime_benchmark = WalltimeBenchmark::from_runtime_data(
        bench_name.clone(),
        bench_uri.clone(),
        vec![1; times_per_round_ns.len()],
        times_per_round_ns,
        max_time_ns,
    );

    let walltime_results = WalltimeResults::from_benchmarks(vec![walltime_benchmark])
        .expect("Failed to create walltime results");

    walltime_results
        .save_to_file(
            std::env::var("CODSPEED_PROFILE_FOLDER")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::current_dir().unwrap().join(".codspeed")),
        )
        .expect("Failed to save walltime results");

    Ok(())
}
