use clap::Parser;
use codspeed::instrument_hooks::InstrumentHooks;

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

    // Execute the command
    let status = std::process::Command::new(&args.command[0])
        .args(&args.command[1..])
        .status();

    hooks.stop_benchmark().unwrap();
    hooks.set_executed_benchmark(&bench_name).unwrap();

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
