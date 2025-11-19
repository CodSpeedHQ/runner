use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use heaptrack::Tracker;

#[derive(Parser)]
#[command(name = "heaptrack")]
#[command(about = "Track memory allocations using eBPF", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    /// Track memory allocations for a command
    Track {
        /// Command to execute and track
        command: String,

        /// Arguments for the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,

        /// Output file for allocation data
        #[arg(short, long, default_value = "allocations.jsonl")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::builder()
        .parse_env(env_logger::Env::new().filter_or("CODSPEED_LOG", "info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Track {
            command,
            args,
            output,
        } => track_command(&command, &args, &output)?,
    }

    Ok(())
}

fn track_command(cmd: &str, args: &[String], output_path: &PathBuf) -> Result<()> {
    let file = File::create(output_path).context("Failed to create output file")?;
    let writer = Arc::new(Mutex::new(BufWriter::new(file)));

    let mut tracker = Tracker::new()?;

    // Start the target command
    let mut child = Command::new(cmd)
        .args(args)
        .spawn()
        .context("Failed to spawn command")?;
    let root_pid = child.id() as i32;
    let event_rx = tracker.track(root_pid)?;
    info!("Spawned child with pid {root_pid}");

    // Spawn event processing thread
    let writer_clone = writer.clone();
    let _event_thread = thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            if let Ok(mut w) = writer_clone.lock() {
                let _ = writeln!(w, "{}", serde_json::to_string(&event).unwrap());
            }
        }
    });

    // Wait for the command to complete
    let status = child.wait().context("Failed to wait for command")?;
    info!("Command exited with status: {status}");

    // Flush and close the output file
    if let Ok(mut w) = writer.lock() {
        let _ = w.flush();
    }

    println!("Allocation data written to: {}", output_path.display());

    Ok(())
}
