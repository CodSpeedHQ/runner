use super::perf_map::ProcessSymbols;
use super::unwind_data::UnwindDataExt;
use crate::prelude::*;
use libc::pid_t;
use linux_perf_data::PerfFileReader;
use linux_perf_data::PerfFileRecord;
use linux_perf_data::linux_perf_event_reader::EventRecord;
use linux_perf_data::linux_perf_event_reader::RecordType;
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

    // 1MiB buffer
    let reader = std::io::BufReader::with_capacity(
        1024 * 1024,
        std::fs::File::open(perf_file_path.as_ref())?,
    );

    let PerfFileReader {
        mut perf_file,
        mut record_iter,
    } = PerfFileReader::parse_pipe(reader)?;

    while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
        let PerfFileRecord::EventRecord { record, .. } = record else {
            continue;
        };

        // Check the type from the raw record to avoid parsing overhead since we do not care about
        // most records.
        if record.record_type != RecordType::MMAP2 {
            continue;
        }

        let Ok(parsed_record) = record.parse() else {
            continue;
        };

        // Should never fail since we already checked the type in the raw record
        let EventRecord::Mmap2(record) = parsed_record else {
            continue;
        };

        // Check PROT_EXEC early to avoid string allocation for non-executable mappings
        if record.protection as i32 & libc::PROT_EXEC == 0 {
            continue;
        }

        // Filter on raw bytes before allocating a String
        let path_slice: &[u8] = &record.path.as_slice();

        // Skip anonymous mappings
        if path_slice == b"//anon" {
            continue;
        }

        // Skip special mappings like [vdso], [heap], etc.
        if path_slice.first() == Some(&b'[') && path_slice.last() == Some(&b']') {
            continue;
        }

        let record_path_string = String::from_utf8_lossy(path_slice).into_owned();
        let end_addr = record.address + record.length;

        trace!(
            "Mapping: Pid {}: {:016x}-{:016x} {:08x} {:?} (Prot {:?})",
            record.pid,
            record.address,
            end_addr,
            record.page_offset,
            record_path_string,
            record.protection,
        );
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
                trace!(
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
