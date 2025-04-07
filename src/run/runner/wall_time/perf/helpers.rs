use std::collections::HashMap;

use anyhow::{anyhow, bail};
use linux_perf_data::{linux_perf_event_reader::EventRecord, PerfFileReader, PerfFileRecord};

pub fn find_pid<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<i32> {
    let content = std::fs::read(path.as_ref())?;
    let reader = std::io::Cursor::new(content);

    let PerfFileReader {
        mut record_iter,
        mut perf_file,
    } = PerfFileReader::parse_file(reader)?;

    let mut pid_freq = HashMap::new();
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
        }
    }

    // Choose the pid with the highest frequency. However, we can only use a pid if more than N% of the
    // events are from that pid.
    //
    let total_count = pid_freq.values().sum::<u64>();
    let (pid, pid_count) = pid_freq
        .iter()
        .max_by_key(|&(_, count)| count)
        .ok_or_else(|| anyhow!("Couldn't find pid in perf.data"))?;
    log::debug!("Pid frequency: {:?}", pid_freq);

    let pid_percentage = (*pid_count as f64 / total_count as f64) * 100.0;
    if pid_percentage < 75.0 {
        bail!(
            "Pid {} has only {:.2}% of total events",
            pid,
            pid_percentage
        );
    }

    Ok(*pid)
}
