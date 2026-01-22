use crate::prelude::*;

use crate::BenchmarkCommand;
use crate::constants;
use crate::uri;
use codspeed::instrument_hooks::InstrumentHooks;
use std::process::Command;

pub fn perform(commands: Vec<BenchmarkCommand>) -> Result<()> {
    let hooks = InstrumentHooks::instance();

    for benchmark_cmd in commands {
        let name_and_uri = uri::generate_name_and_uri(&benchmark_cmd.name, &benchmark_cmd.command);
        name_and_uri.print_executing();

        let mut cmd = Command::new(&benchmark_cmd.command[0]);
        cmd.args(&benchmark_cmd.command[1..]);
        hooks.start_benchmark().unwrap();
        let status = cmd.status();
        hooks.stop_benchmark().unwrap();
        let status = status.context("Failed to execute command")?;

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }

        hooks.set_executed_benchmark(&name_and_uri.uri).unwrap();
    }

    Ok(())
}

/// Path to the preload library built during compilation.
/// This library is used for LD_PRELOAD-based instrumentation injection.
pub const PRELOAD_LIB_PATH: &str = env!("CODSPEED_PRELOAD_LIB_PATH");

pub fn perform_with_valgrind(commands: Vec<BenchmarkCommand>) -> Result<()> {
    for benchmark_cmd in commands {
        let name_and_uri = uri::generate_name_and_uri(&benchmark_cmd.name, &benchmark_cmd.command);
        name_and_uri.print_executing();

        let mut cmd = Command::new(&benchmark_cmd.command[0]);
        cmd.args(&benchmark_cmd.command[1..]);
        // Use LD_PRELOAD to inject instrumentation into the child process
        cmd.env("LD_PRELOAD", PRELOAD_LIB_PATH);
        cmd.env(constants::URI_ENV, &name_and_uri.uri);

        let status = cmd.status().context("Failed to execute command")?;

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }
    }

    Ok(())
}
