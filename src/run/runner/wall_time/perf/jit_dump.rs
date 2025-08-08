use crate::{prelude::*, run::runner::wall_time::perf::unwind_data::UnwindData};
use linux_perf_data::jitdump::{JitDumpReader, JitDumpRecord};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

struct JitDump {
    path: PathBuf,
}

impl JitDump {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Parses the JIT dump file and converts it into a list of `UnwindData`.
    ///
    /// The JIT dump file contains synthetic `eh_frame` data for jitted functions. This can be parsed and
    /// then converted to `UnwindData` which is used for stack unwinding.
    ///
    /// See: https://github.com/python/cpython/blob/main/Python/perf_jit_trampoline.c
    pub fn into_unwind_data(self) -> Result<Vec<UnwindData>> {
        let file = std::fs::File::open(self.path)?;

        let mut jit_unwind_data = Vec::new();
        let mut current_unwind_info: Option<(Vec<u8>, Vec<u8>)> = None;

        let mut reader = JitDumpReader::new(file)?;
        while let Some(raw_record) = reader.next_record()? {
            // The first recording is always the unwind info, followed by the code load event
            // (see `perf_map_jit_write_entry` in https://github.com/python/cpython/blob/9743d069bd53e9d3a8f09df899ec1c906a79da24/Python/perf_jit_trampoline.c#L1163C13-L1163C37)
            match raw_record.parse()? {
                JitDumpRecord::CodeLoad(record) => {
                    let name = record.function_name.as_slice();
                    let name = String::from_utf8_lossy(&name);

                    let avma_start = record.vma;
                    let code_size = record.code_bytes.len() as u64;
                    let avma_end = avma_start + code_size;

                    let Some((eh_frame, eh_frame_hdr)) = current_unwind_info.take() else {
                        warn!("No unwind info available for JIT code load: {name}");
                        continue;
                    };

                    jit_unwind_data.push(UnwindData {
                        path: format!("jit_{name}"),
                        avma_range: avma_start..avma_end,
                        base_avma: 0,
                        eh_frame_hdr,
                        eh_frame_hdr_svma: 0..0,
                        eh_frame,
                        eh_frame_svma: 0..0,
                    });
                }
                JitDumpRecord::CodeUnwindingInfo(record) => {
                    // Store unwind info for the next code loads
                    current_unwind_info = Some((
                        record.eh_frame.as_slice().to_vec(),
                        record.eh_frame_hdr.as_slice().to_vec(),
                    ));
                }
                _ => {
                    warn!("Unhandled JIT dump record: {raw_record:?}");
                }
            }
        }

        Ok(jit_unwind_data)
    }
}

/// Converts all the `jit-<pid>.dump` into unwind data and copies it to the profile folder.
pub async fn harvest_perf_jit_for_pids(profile_folder: &Path, pids: &HashSet<i32>) -> Result<()> {
    for pid in pids {
        let name = format!("jit-{pid}.dump");
        let path = PathBuf::from("/tmp").join(&name);

        if !path.exists() {
            continue;
        }
        debug!("Found JIT dump file: {path:?}");

        let unwind_data = match JitDump::new(path).into_unwind_data() {
            Ok(unwind_data) => unwind_data,
            Err(error) => {
                warn!("Failed to convert jit dump into unwind data: {error:?}");
                continue;
            }
        };

        for module in unwind_data {
            module.save_to(profile_folder, *pid as _)?;
        }
    }

    Ok(())
}
