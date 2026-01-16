use crate::prelude::*;

use crate::exec_targets::ExecTarget;
use crate::uri;
use crate::uri::NameAndUri;
use codspeed::instrument_hooks::InstrumentHooks;
use std::process::Command;

pub fn perform(name_and_uri: NameAndUri, command: Vec<String>) -> Result<()> {
    let hooks = InstrumentHooks::instance();

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);
    hooks.start_benchmark().unwrap();
    let status = cmd.status();
    hooks.stop_benchmark().unwrap();
    let status = status.context("Failed to execute command")?;

    if !status.success() {
        bail!("Command exited with non-zero status: {status}");
    }

    hooks.set_executed_benchmark(&name_and_uri.uri).unwrap();

    Ok(())
}

/// Run multiple targets sequentially for analysis mode
pub fn perform_targets(targets: Vec<ExecTarget>) -> Result<()> {
    let hooks = InstrumentHooks::instance();

    for (idx, target) in targets.iter().enumerate() {
        let name_and_uri = uri::generate_name_and_uri(&target.name, &target.command);

        info!(
            "Running target {}/{}: {}",
            idx + 1,
            targets.len(),
            name_and_uri.name
        );

        let mut cmd = Command::new(&target.command[0]);
        cmd.args(&target.command[1..]);
        hooks.start_benchmark().unwrap();
        let status = cmd.status();
        hooks.stop_benchmark().unwrap();
        let status = status.context("Failed to execute command")?;

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }

        hooks.set_executed_benchmark(&name_and_uri.uri).unwrap();
    }

    info!("Completed {} targets", targets.len());

    Ok(())
}
