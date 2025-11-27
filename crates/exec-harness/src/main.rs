use crate::walltime_results::WalltimeResults;
use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;
use codspeed::walltime_results::WalltimeBenchmark;
use nix::libc::pid_t;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::path::PathBuf;

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

    // TODO: Change this
    hooks
        .set_integration("codspeed-rust", env!("CARGO_PKG_VERSION"))
        .unwrap();
    hooks.start_benchmark().unwrap();

    // Run 10 iterations
    const NUM_ITERATIONS: usize = 10;
    let mut times_per_round_ns = Vec::with_capacity(NUM_ITERATIONS);

    for _ in 0..NUM_ITERATIONS {
        // Spawn the command
        let mut child = match std::process::Command::new(&args.command[0])
            .args(&args.command[1..])
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn command: {e}");
                std::process::exit(1);
            }
        };

        // Get the PID
        let pid = child.id() as pid_t;
        let nix_pid = Pid::from_raw(pid);

        // Stop the process
        if let Err(e) = signal::kill(nix_pid, Signal::SIGSTOP) {
            eprintln!("Failed to send SIGSTOP to process {pid}: {e}");
            std::process::exit(1);
        }

        // TODO: Do something with the PID
        hooks
            .set_executed_benchmark_with_pid(&bench_name, pid)
            .unwrap();

        // Start monotonic timer for this iteration
        let bench_start = InstrumentHooks::current_timestamp();
        // Resume the process
        if let Err(e) = signal::kill(nix_pid, Signal::SIGCONT) {
            eprintln!("Failed to send SIGCONT to process {pid}: {e}");
            std::process::exit(1);
        }

        // Wait for the process to complete
        let status = match child.wait() {
            Ok(status) => status,
            Err(e) => {
                eprintln!("Failed to wait for command: {e}");
                std::process::exit(1);
            }
        };

        // Measure elapsed time
        let bench_end = InstrumentHooks::current_timestamp();
        hooks.add_benchmark_timestamps(bench_start, bench_end);

        // Exit immediately if any iteration fails
        if !status.success() {
            eprintln!("Command failed with exit code: {:?}", status.code());
            std::process::exit(status.code().unwrap_or(1));
        }

        // Calculate and store the elapsed time in nanoseconds
        let elapsed_ns = (bench_end - bench_start) as u128;
        times_per_round_ns.push(elapsed_ns);
    }

    hooks.stop_benchmark().unwrap();

    // Collect walltime results
    // 10 iterations: 10 rounds with 1 iteration each
    let max_time_ns = times_per_round_ns.iter().copied().max();
    let walltime_benchmark = WalltimeBenchmark::from_runtime_data(
        bench_name.clone(),      // name
        bench_name.clone(),      // uri (using name as uri)
        vec![1; NUM_ITERATIONS], // 1 iteration per round for each of 10 rounds
        times_per_round_ns,      // timing for each round
        max_time_ns,             // max time across all rounds
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

    // All iterations succeeded
    std::process::exit(0);
}
