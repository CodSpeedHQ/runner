use anyhow::Context;

use crate::run::runner::helpers::setup::run_with_sudo;

pub mod fifo;
pub mod perf_map;
pub mod unwind_data;

pub fn setup_environment() -> anyhow::Result<()> {
    let sysctl_read = |name: &str| -> anyhow::Result<i64> {
        let output = std::process::Command::new("sysctl").arg(name).output()?;
        let output = String::from_utf8(output.stdout)?;

        Ok(output
            .split(" = ")
            .last()
            .context("Couldn't find the value in sysctl output")?
            .trim()
            .parse::<i64>()?)
    };

    // Allow access to kernel symbols
    if sysctl_read("kernel.kptr_restrict")? != 0 {
        run_with_sudo(&["sysctl", "-w", "kernel.kptr_restrict=0"])?;
    }

    // Allow non-root profiling
    if sysctl_read("kernel.perf_event_paranoid")? != 1 {
        run_with_sudo(&["sysctl", "-w", "kernel.perf_event_paranoid=1"])?;
    }

    Ok(())
}
