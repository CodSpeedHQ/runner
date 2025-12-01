use crate::walltime::WalltimeResults;
use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;
use codspeed::walltime_results::WalltimeBenchmark;
use std::path::PathBuf;
use std::process;

mod walltime;

#[derive(Parser, Debug)]
#[command(name = "exec-harness")]
#[command(about = "CodSpeed exec harness - wraps commands with performance instrumentation")]
struct Args {
    /// Optional benchmark name (defaults to command filename)
    #[arg(long)]
    name: Option<String>,

    /// The command and arguments to execute
    command: Vec<String>,
}

fn main() {
    let args = Args::parse();

    if args.command.is_empty() {
        eprintln!("Error: No command provided");
        process::exit(1);
    }

    // Derive benchmark name from command if not provided
    let bench_name = args.name.unwrap_or_else(|| {
        // Extract filename from command path
        let cmd = &args.command[0];
        std::path::Path::new(cmd)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "exec_benchmark".to_string())
    });

    let hooks = InstrumentHooks::instance();

    // TODO: Change this to avoid impersonating `codspeed-rust`
    hooks
        .set_integration("codspeed-rust", env!("CARGO_PKG_VERSION"))
        .unwrap();

    const NUM_ITERATIONS: usize = 10;
    let mut times_per_round_ns = Vec::with_capacity(NUM_ITERATIONS);

    hooks.start_benchmark().unwrap();
    for _ in 0..NUM_ITERATIONS {
        // Start monotonic timer for this iteration
        let bench_start = InstrumentHooks::current_timestamp();

        // Spawn the command
        let mut child = match process::Command::new(&args.command[0])
            .args(&args.command[1..])
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn command: {e}");
                process::exit(1);
            }
        };
        // Wait for the process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => {
                eprintln!("Failed to wait for command: {e}");
                process::exit(1);
            }
        };

        // Measure elapsed time
        let bench_end = InstrumentHooks::current_timestamp();
        hooks.add_benchmark_timestamps(bench_start, bench_end);

        // Exit immediately if any iteration fails
        if !status.success() {
            eprintln!("Command failed with exit code: {:?}", status.code());
            process::exit(status.code().unwrap_or(1));
        }

        // Calculate and store the elapsed time in nanoseconds
        let elapsed_ns = (bench_end - bench_start) as u128;
        times_per_round_ns.push(elapsed_ns);
    }

    hooks.stop_benchmark().unwrap();
    hooks.set_executed_benchmark(&bench_name).unwrap();

    // Collect walltime results
    let max_time_ns = times_per_round_ns.iter().copied().max();
    let walltime_benchmark = WalltimeBenchmark::from_runtime_data(
        bench_name.clone(),
        format!("standalone_run::{bench_name}"),
        vec![1; NUM_ITERATIONS],
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
}
