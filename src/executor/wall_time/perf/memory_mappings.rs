use super::perf_map::ProcessSymbols;
use super::unwind_data::UnwindDataExt;
use crate::prelude::*;
use libc::pid_t;
use runner_shared::unwind_data::UnwindData;
use std::collections::HashMap;

#[cfg(target_os = "linux")]
pub(super) fn process_memory_mappings(
    pid: pid_t,
    symbols_by_pid: &mut HashMap<pid_t, ProcessSymbols>,
    unwind_data_by_pid: &mut HashMap<pid_t, Vec<UnwindData>>,
) -> anyhow::Result<()> {
    use procfs::process::MMPermissions;
    let bench_proc =
        procfs::process::Process::new(pid as _).expect("Failed to find benchmark process");
    let exe_maps = bench_proc.maps().expect("Failed to read /proc/{pid}/maps");

    debug!("Process memory mappings for PID {pid}:");
    for map in exe_maps.iter().sorted_by_key(|m| m.address.0) {
        let (base_addr, end_addr) = map.address;
        debug!(
            "  {:016x}-{:016x} {:08x} {:?} {:?} ",
            base_addr, end_addr, map.offset, map.pathname, map.perms,
        );
    }

    for map in &exe_maps {
        let page_offset = map.offset;
        let (base_addr, end_addr) = map.address;
        let path = match &map.pathname {
            procfs::process::MMapPath::Path(path) => Some(path.clone()),
            _ => None,
        };

        let Some(path) = &path else {
            if map.perms.contains(MMPermissions::EXECUTE) {
                debug!("Found executable mapping without path: {base_addr:x} - {end_addr:x}");
            }
            continue;
        };

        if !map.perms.contains(MMPermissions::EXECUTE) {
            continue;
        }

        symbols_by_pid
            .entry(pid)
            .or_insert(ProcessSymbols::new(pid))
            .add_mapping(pid, path, base_addr, end_addr, map.offset);
        debug!("Added mapping for module {path:?}");

        match UnwindData::new(
            path.to_string_lossy().as_bytes(),
            page_offset,
            base_addr,
            end_addr,
            None,
        ) {
            Ok(unwind_data) => {
                unwind_data_by_pid.entry(pid).or_default().push(unwind_data);
                debug!("Added unwind data for {path:?} ({base_addr:x} - {end_addr:x})");
            }
            Err(error) => {
                debug!(
                    "Failed to create unwind data for module {}: {}",
                    path.display(),
                    error
                );
            }
        }
    }

    Ok(())
}
