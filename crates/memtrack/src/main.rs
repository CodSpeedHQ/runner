use anyhow::{Context, Result, anyhow};
use clap::Parser;
use ipc_channel::ipc::{self};
use log::{debug, info};
use memtrack::{MemtrackIpcMessage, Tracker, handle_ipc_message};
use runner_shared::artifacts::{ArtifactExt, MemtrackArtifact, MemtrackEvent};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Parser)]
#[command(name = "memtrack")]
#[command(version, about = "Track memory allocations using eBPF", long_about = None)]
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

        /// Output folder for the allocations data
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Optional IPC server name for receiving control commands
        #[arg(long)]
        ipc_server: Option<String>,
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
            output: out_dir,
            ipc_server,
        } => {
            debug!("Starting memtrack for command: {command}");

            let (root_pid, events, status) =
                track_command(&command, ipc_server).context("Failed to track command")?;
            let result = MemtrackArtifact { events };
            result.save_with_pid_to(&out_dir, root_pid as libc::pid_t)?;

            std::process::exit(status.code().unwrap_or(1));
        }
    }
}

fn track_command(
    cmd_string: &str,
    ipc_server_name: Option<String>,
) -> anyhow::Result<(u32, Vec<MemtrackEvent>, std::process::ExitStatus)> {
    let tracker = Tracker::new()?;

    let tracker_arc = Arc::new(Mutex::new(tracker));
    let ipc_handle = if let Some(server_name) = ipc_server_name {
        debug!("Connecting to IPC server: {server_name}");

        let (tx, rx) = ipc::channel::<MemtrackIpcMessage>()?;
        let sender = ipc::IpcSender::connect(server_name)?;
        sender.send(tx)?;

        let tracker_clone = tracker_arc.clone();
        Some(thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                handle_ipc_message(msg, &tracker_clone);
            }
        }))
    } else {
        None
    };

    // Start the target command using bash to handle shell syntax
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(cmd_string)
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn child process: {e}"))?;
    let root_pid = child.id() as i32;
    let event_rx = { tracker_arc.lock().unwrap().track(root_pid)? };
    info!("Spawned child with pid {root_pid}");

    // Spawn event processing thread
    let process_events = Arc::new(AtomicBool::new(true));
    let process_events_clone = process_events.clone();
    let processing_thread = thread::spawn(move || {
        let mut events = Vec::new();
        loop {
            if !process_events_clone.load(Ordering::Relaxed) {
                break;
            }

            let Ok(event) = event_rx.try_recv() else {
                continue;
            };

            events.push(event.into());
        }
        events
    });

    // Wait for the command to complete
    let status = child.wait().context("Failed to wait for command")?;
    info!("Command exited with status: {status}");

    info!("Waiting for the event processing thread to finish");
    process_events.store(false, Ordering::Relaxed);
    let events = processing_thread
        .join()
        .map_err(|_| anyhow::anyhow!("Failed to join event thread"))?;

    // IPC thread will exit when channel closes
    drop(ipc_handle);

    Ok((root_pid as u32, events, status))
}
