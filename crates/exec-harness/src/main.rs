use crate::walltime_results::WalltimeResults;
use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;
use codspeed::walltime_results::WalltimeBenchmark;
use std::path::PathBuf;
use std::time::Instant;

mod walltime_results;

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
        std::process::exit(1);
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

    hooks
        .set_integration("codspeed-exec", env!("CARGO_PKG_VERSION"))
        .unwrap();
    hooks.start_benchmark().unwrap();

    // Start monotonic timer
    let start = Instant::now();

    // Execute the command
    let status = std::process::Command::new(&args.command[0])
        .args(&args.command[1..])
        .status();

    // Measure elapsed time
    let elapsed = start.elapsed();
    let elapsed_ns = elapsed.as_nanos();

    hooks.stop_benchmark().unwrap();
    hooks.set_executed_benchmark(&bench_name).unwrap();

    // Collect walltime results
    // Single execution: 1 round with 1 iteration
    let walltime_benchmark = WalltimeBenchmark::from_runtime_data(
        bench_name.clone(), // name
        bench_name.clone(), // uri (using name as uri)
        vec![1],            // 1 iteration per round
        vec![elapsed_ns],   // timing for the single round
        Some(elapsed_ns),   // Max
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

    // Propagate exit code
    match status {
        Ok(exit_status) => {
            std::process::exit(exit_status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Failed to execute command: {e}");
            std::process::exit(1);
        }
    }
}
