use crate::prelude::*;

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
