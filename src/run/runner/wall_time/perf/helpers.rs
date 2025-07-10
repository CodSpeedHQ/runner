use crate::prelude::*;
use linux_perf_data::{PerfFileReader, PerfFileRecord, linux_perf_event_reader::EventRecord};
use std::collections::HashMap;

/// Tries to find the pid of the sampled process within a perf.data file.
pub fn find_pid<P: AsRef<std::path::Path>>(perf_file: P) -> anyhow::Result<i32> {
    let content = std::fs::read(perf_file.as_ref())?;
    let reader = std::io::Cursor::new(content);

    let PerfFileReader {
        mut record_iter,
        mut perf_file,
    } = PerfFileReader::parse_file(reader)?;

    let mut pid_freq = HashMap::new();

    // Only consider the first N events to reduce the performance impact. Certain benchmark libraries can generate
    // more than 100k for each benchmark, which can slow down the runner a lot. The highest chance of finding
    // different pids is in the first few events, where there's a possible overlap.
    const COUNT_FIRST_N: usize = 1000;
    let mut i = 0;

    while let Some(record) = record_iter.next_record(&mut perf_file)? {
        let PerfFileRecord::EventRecord { record, .. } = record else {
            continue;
        };

        let Ok(parsed_record) = record.parse() else {
            continue;
        };

        let EventRecord::Sample(event) = parsed_record else {
            continue;
        };

        // Ignore kernel events
        if event.pid == Some(-1) {
            continue;
        }

        if let Some(pid) = event.pid {
            *pid_freq.entry(pid).or_insert(0) += 1;

            i += 1;
            if i >= COUNT_FIRST_N {
                break;
            }
        }
    }
    debug!("Pid frequency: {pid_freq:?}");

    // Choose the pid with the highest frequency. However, we can only use a pid if more than N% of the
    // events are from that pid.
    let total_count = pid_freq.values().sum::<u64>();
    let (pid, pid_count) = pid_freq
        .iter()
        .max_by_key(|&(_, count)| count)
        .ok_or_else(|| anyhow!("Couldn't find pid in perf.data"))?;
    log::debug!("Pid frequency: {pid_freq:?}");

    let pid_percentage = (*pid_count as f64 / total_count as f64) * 100.0;
    if pid_percentage < 75.0 {
        bail!(
            "Most common pid {} only has {:.2}% of total events",
            pid,
            pid_percentage
        );
    }

    Ok(*pid)
}
