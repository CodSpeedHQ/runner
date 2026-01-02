use crate::prelude::*;
use crate::walltime::WalltimeResults;
use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;
use codspeed::walltime_results::WalltimeBenchmark;
use std::path::PathBuf;

mod prelude;
mod walltime;

#[derive(Parser, Debug)]
#[command(name = "exec-harness")]
#[command(
    version,
    about = "CodSpeed exec harness - wraps commands with performance instrumentation"
)]
struct Args {
    /// Optional benchmark name (defaults to command filename)
    #[arg(long)]
    name: Option<String>,

    /// The command and arguments to execute
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

    // Derive benchmark name from command if not provided
    let bench_name = args.name.unwrap_or_else(|| {
        // Extract filename from command path
        let cmd = &args.command[0];
        std::path::Path::new(cmd).to_string_lossy().into_owned()
    });

    // TODO: Better URI generation
    let bench_uri = format!("standalone_run::{bench_name}");

    let hooks = InstrumentHooks::instance();

    // TODO: Stop impersonating codspeed-rust 🥸
    hooks
        .set_integration("codspeed-rust", env!("CARGO_PKG_VERSION"))
        .unwrap();

    let times_per_round_ns = walltime::perform(
        bench_uri.clone(),
        args.command,
        &walltime::ExecutionOptions::default(),
    )?;

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
