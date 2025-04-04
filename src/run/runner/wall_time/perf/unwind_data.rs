// !!!!!!!!!!!!!!!!!!!!!
// !!! DO NOT MODIFY !!!
// !!!!!!!!!!!!!!!!!!!!!
//
// This file has to be in sync with perf-parser!

use anyhow::{bail, Context};
use debugid::CodeId;
use linux_perf_data::{
    linux_perf_event_reader::{EventRecord, Mmap2FileId},
    DsoKey, PerfFileReader, PerfFileRecord,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Unwind data for a single module.
#[derive(Debug, Serialize, Deserialize)]
pub struct UnwindData {
    pub path: String,

    pub avma_range: Range<u64>,
    pub base_avma: u64,

    pub eh_frame_hdr: Vec<u8>,
    pub eh_frame_hdr_svma: Range<u64>,

    pub eh_frame: Vec<u8>,
    pub eh_frame_svma: Range<u64>,
}

impl UnwindData {
    // Based on this: https://github.com/mstange/linux-perf-stuff/blob/22ca6531b90c10dd2a4519351c843b8d7958a451/src/main.rs#L747-L893
    fn new(
        path_slice: &[u8],
        mapping_start_file_offset: u64,
        mapping_start_avma: u64,
        mapping_size: u64,
        build_id: Option<&[u8]>,
    ) -> anyhow::Result<Self> {
        use object::{Object, ObjectSection, ObjectSegment};

        let avma_range = mapping_start_avma..(mapping_start_avma + mapping_size);

        let path = String::from_utf8_lossy(path_slice).to_string();
        let Some(file) = std::fs::File::open(&path).ok() else {
            bail!("Could not open file {path}");
        };

        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
        let file = object::File::parse(&mmap[..])?;

        // Verify the build id (if we have one)
        match (build_id, file.build_id()) {
            (Some(build_id), Ok(Some(file_build_id))) => {
                if build_id != file_build_id {
                    let file_build_id = CodeId::from_binary(file_build_id);
                    let expected_build_id = CodeId::from_binary(build_id);
                    bail!(
                        "File {:?} has non-matching build ID {} (expected {})",
                        path,
                        file_build_id,
                        expected_build_id
                    );
                }
            }
            (Some(_), Err(_)) | (Some(_), Ok(None)) => {
                bail!(
                    "File {:?} does not contain a build ID, but we expected it to have one",
                    path
                );
            }
            _ => {
                // No build id to check
            }
        };

        let mapping_end_file_offset = mapping_start_file_offset + mapping_size;
        let mapped_segment = file
            .segments()
            .find(|segment| {
                let (segment_start_file_offset, segment_size) = segment.file_range();
                let segment_end_file_offset = segment_start_file_offset + segment_size;
                mapping_start_file_offset <= segment_start_file_offset
                    && segment_end_file_offset <= mapping_end_file_offset
            })
            .context("Failed to find segment")?;

        let (segment_start_file_offset, _segment_size) = mapped_segment.file_range();
        let segment_start_svma = mapped_segment.address();
        let segment_start_avma =
            mapping_start_avma + (segment_start_file_offset - mapping_start_file_offset);

        let base_avma = segment_start_avma - segment_start_svma;
        let eh_frame = file.section_by_name(".eh_frame");
        let eh_frame_hdr = file.section_by_name(".eh_frame_hdr");

        fn section_data<'a>(section: &impl ObjectSection<'a>) -> Option<Vec<u8>> {
            section.data().ok().map(|data| data.to_owned())
        }

        let eh_frame_data = eh_frame.as_ref().and_then(section_data);
        let eh_frame_hdr_data = eh_frame_hdr.as_ref().and_then(section_data);

        fn svma_range<'a>(section: &impl ObjectSection<'a>) -> Range<u64> {
            section.address()..section.address() + section.size()
        }

        Ok(Self {
            path,
            avma_range,
            base_avma,
            eh_frame_hdr: eh_frame_hdr_data.context("Failed to find eh_frame hdr data")?,
            eh_frame_hdr_svma: eh_frame_hdr
                .as_ref()
                .map(svma_range)
                .context("Failed to find eh_frame hdr section")?,
            eh_frame: eh_frame_data.context("Failed to find eh_frame data")?,
            eh_frame_svma: eh_frame
                .as_ref()
                .map(svma_range)
                .context("Failed to find eh_frame section")?,
        })
    }

    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut writer = std::fs::File::create(path.as_ref())?;
        bincode::serialize_into(&mut writer, self)?;
        Ok(())
    }

    pub fn name(&self) -> String {
        match self.path.rfind('/') {
            Some(pos) => self.path[pos + 1..].to_owned(),
            None => self.path.clone(),
        }
    }
}

#[derive(Debug)]
pub struct UnwindDataLoader {
    modules: Vec<UnwindData>,
}

impl UnwindDataLoader {
    pub fn from_perf_file<P: AsRef<std::path::Path>>(path: P) -> Option<Self> {
        let content = std::fs::read(path.as_ref()).unwrap();
        let reader = std::io::Cursor::new(content);

        let PerfFileReader {
            mut record_iter,
            mut perf_file,
        } = PerfFileReader::parse_file(reader).ok()?;
        let build_ids = perf_file.build_ids().ok().unwrap_or_default();

        let find_build_id = |path: &[u8], cpu_mode| -> Option<Option<Vec<u8>>> {
            let dso_key = DsoKey::detect(path, cpu_mode)?;
            let build_id = build_ids
                .get(&dso_key)
                .map(|db| &db.build_id[..])
                .map(Vec::from);

            Some(build_id)
        };

        let mut modules = Vec::new();
        while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
            let PerfFileRecord::EventRecord { record, .. } = record else {
                continue;
            };

            let Ok(parsed_record) = record.parse() else {
                continue;
            };

            let (is_exec, path, page_offset, addr, length, build_id) = match parsed_record {
                EventRecord::Mmap(event) => {
                    let Some(build_id) = find_build_id(&event.path.as_slice(), event.cpu_mode)
                    else {
                        continue;
                    };

                    // Ignore kernel mappings
                    if event.pid == -1 {
                        continue;
                    }

                    (
                        event.is_executable,
                        event.path.as_slice(),
                        event.page_offset,
                        event.address,
                        event.length,
                        build_id,
                    )
                }
                EventRecord::Mmap2(event) => {
                    let build_id = if let Mmap2FileId::BuildId(build_id) = event.file_id {
                        Some(build_id)
                    } else {
                        let Some(build_id) = find_build_id(&event.path.as_slice(), event.cpu_mode)
                        else {
                            continue;
                        };
                        build_id
                    };

                    (
                        true,
                        event.path.as_slice(),
                        event.page_offset,
                        event.address,
                        event.length,
                        build_id,
                    )
                }
                _ => {
                    continue;
                }
            };

            if !is_exec {
                continue;
            }

            if let Ok(module) =
                UnwindData::new(&path, page_offset, addr, length, build_id.as_deref())
            {
                modules.push(module);
            }
        }

        Some(Self { modules })
    }

    pub fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        for module in &self.modules {
            let path = folder.as_ref().join(format!(
                "{}_{:x}_{:x}.unwind",
                module.name(),
                module.avma_range.start,
                module.avma_range.end
            ));
            module.to_file(path)?;
        }

        Ok(())
    }
}
