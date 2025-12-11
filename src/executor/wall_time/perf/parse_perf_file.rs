use super::perf_map::ProcessSymbols;
use super::unwind_data::UnwindDataExt;
use crate::executor::helpers::run_with_sudo::run_with_sudo;
use crate::prelude::*;
use libc::pid_t;
use linux_perf_data::PerfFileReader;
use linux_perf_data::PerfFileRecord;
use linux_perf_data::linux_perf_event_reader::EventRecord;
use runner_shared::unwind_data::UnwindData;
use std::collections::HashMap;
use std::path::Path;

pub struct MemmapRecordsOutput {
    pub symbols_by_pid: HashMap<pid_t, ProcessSymbols>,
    pub unwind_data_by_pid: HashMap<pid_t, Vec<UnwindData>>,
}

pub(super) fn parse_for_memmap2<P: AsRef<Path>>(perf_file_path: P) -> Result<MemmapRecordsOutput> {
    let mut symbols_by_pid = HashMap::<pid_t, ProcessSymbols>::new();
    let mut unwind_data_by_pid = HashMap::<pid_t, Vec<UnwindData>>::new();

    //FIXME: Remove this once again when we parse directly from pipedata
    {
        let tmp_perf_file_path = perf_file_path.as_ref().to_string_lossy();

        // We ran perf with sudo, so we have to change the ownership of the perf.data
        run_with_sudo(
            "chown",
            [
                "-R",
                &format!(
                    "{}:{}",
                    nix::unistd::Uid::current(),
                    nix::unistd::Gid::current()
                ),
                &tmp_perf_file_path,
            ],
        )?;
    }
    let reader = std::fs::File::open(perf_file_path.as_ref()).unwrap();

    let PerfFileReader {
        mut perf_file,
        mut record_iter,
    } = PerfFileReader::parse_file(reader)?;

    while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
        let PerfFileRecord::EventRecord { record, .. } = record else {
            continue;
        };

        let Ok(parsed_record) = record.parse() else {
            continue;
        };

        let EventRecord::Mmap2(record) = parsed_record else {
            continue;
        };

        let record_path_string = {
            let path_slice = record.path.as_slice();
            String::from_utf8_lossy(&path_slice).into_owned()
        };

        let end_addr = record.address + record.length;

        if record_path_string == "//anon" {
            // Skip anonymous mappings
            trace!(
                "Skipping anonymous mapping: {:x}-{:x}",
                record.address, end_addr
            );
            continue;
        }

        if record_path_string.starts_with("[") && record_path_string.ends_with("]") {
            // Skip special mappings
            trace!(
                "Skipping special mapping: {} - {:x}-{:x}",
                record_path_string, record.address, end_addr
            );
            continue;
        }

        debug!(
            "Pid {}: {:016x}-{:016x} {:08x} {:?} (Prot {:?})",
            record.pid,
            record.address,
            end_addr,
            record.page_offset,
            record_path_string,
            record.protection,
        );

        if record.protection as i32 & libc::PROT_EXEC == 0 {
            continue;
        }

        symbols_by_pid
            .entry(record.pid)
            .or_insert(ProcessSymbols::new(record.pid))
            .add_mapping(
                record.pid,
                &record_path_string,
                record.address,
                end_addr,
                record.page_offset,
            );
        debug!("Added symbols mapping for module {record_path_string:?}");

        match UnwindData::new(
            record_path_string.as_bytes(),
            record.page_offset,
            record.address,
            end_addr,
            None,
        ) {
            Ok(unwind_data) => {
                unwind_data_by_pid
                    .entry(record.pid)
                    .or_default()
                    .push(unwind_data);
                debug!(
                    "Added unwind data for {record_path_string} ({:x} - {:x})",
                    record.address, end_addr
                );
            }
            Err(error) => {
                debug!("Failed to create unwind data for module {record_path_string}: {error}");
            }
        }
    }

    Ok(MemmapRecordsOutput {
        symbols_by_pid,
        unwind_data_by_pid,
    })
}
