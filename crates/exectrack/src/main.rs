use anyhow::Result;
use clap::Parser;
use exectrack::{HierarchyBuilder, Tracker};
use log::{debug, info};
use runner_shared::artifacts::ArtifactExt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "codspeed-exectrack")]
#[command(about = "Track process execution tree using eBPF")]
#[command(version)]
struct Cli {
    /// Command to run and track
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,

    /// Output folder for the process hierarchy data
    #[arg(short, long, default_value = ".")]
    output: PathBuf,
}

fn main() -> Result<()> {
    env_logger::builder()
        .parse_env(env_logger::Env::new().filter_or("CODSPEED_LOG", "info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    track_command(&cli.command, &cli.output)
}

fn track_command(command: &[String], output_dir: &PathBuf) -> Result<()> {
    info!("Starting exectrack for command: {command:?}");

    let mut tracker = Tracker::new()?;

    // FIXME: Start this with SIGKILL
    let mut child = Command::new(&command[0]).args(&command[1..]).spawn()?;

    let root_pid = child.id() as i32;
    let events = tracker.track(root_pid)?;
    let process_thread = std::thread::spawn(move || {
        let mut builder = HierarchyBuilder::new(root_pid);
        for event in events {
            builder.process_event(&event);
        }
        builder.into_hierarchy()
    });
    let status = child.wait()?;
    debug!("Process exited with status: {status:?}");

    // Drop the tracker to close the event channel and allow the event processing thread to complete
    drop(tracker);

    // Print and save the process hierarchy
    let hierarchy = process_thread.join().unwrap();

    debug!("Process hierarchy: {hierarchy:#?}");
    hierarchy.save_with_pid_to(output_dir, root_pid)?;

    Ok(())
}
