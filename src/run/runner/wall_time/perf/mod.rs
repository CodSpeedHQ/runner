use std::collections::HashMap;

use anyhow::Context;
use perf_map::ProcessSymbols;
use unwind_data::UnwindData;

use crate::run::runner::helpers::setup::run_with_sudo;

mod shared;
pub use shared::*;

pub mod fifo;
pub mod helpers;
pub mod perf_map;
pub mod unwind_data;

pub struct BenchmarkData {
    pub(crate) bench_order_by_pid: HashMap<u32, Vec<String>>,
    pub(crate) symbols_by_pid: HashMap<u32, ProcessSymbols>,
    pub(crate) unwind_data_by_pid: HashMap<u32, Vec<UnwindData>>,
}

impl BenchmarkData {
    pub fn save_to<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        for proc_sym in self.symbols_by_pid.values() {
            proc_sym.save_to(&path).unwrap();
        }

        for (pid, modules) in &self.unwind_data_by_pid {
            for module in modules {
                module.save_to(&path, *pid).unwrap();
            }
        }

        for (pid, orders) in &self.bench_order_by_pid {
            let dst_file_name = format!("{}.bench_order", pid);
            let dst_path = path.as_ref().join(dst_file_name);
            std::fs::write(dst_path, orders.join("\n"))?;
        }

        Ok(())
    }

    pub fn bench_count(&self) -> usize {
        self.bench_order_by_pid.values().map(|v| v.len()).sum()
    }
}

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
    if sysctl_read("kernel.perf_event_paranoid")? != -1 {
        run_with_sudo(&["sysctl", "-w", "kernel.perf_event_paranoid=-1"])?;
    }

    Ok(())
}
