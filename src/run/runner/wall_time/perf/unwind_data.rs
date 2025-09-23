//! WARNING: This file has to be in sync with perf-parser!

use anyhow::{Context, bail};
use debugid::CodeId;
use libc::pid_t;
use runner_shared::unwind_data::UnwindData;
use std::ops::Range;

pub trait UnwindDataExt {
    fn new(
        path_slice: &[u8],
        mapping_start_file_offset: u64,
        mapping_start_avma: u64,
        mapping_size: u64,
        build_id: Option<&[u8]>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn save_to<P: AsRef<std::path::Path>>(&self, folder: P, pid: pid_t) -> anyhow::Result<()>;
    fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()>;
}

impl UnwindDataExt for UnwindData {
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
                        "File {path:?} has non-matching build ID {file_build_id} (expected {expected_build_id})"
                    );
                }
            }
            (Some(_), Err(_)) | (Some(_), Ok(None)) => {
                bail!("File {path:?} does not contain a build ID, but we expected it to have one");
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

    fn save_to<P: AsRef<std::path::Path>>(&self, folder: P, pid: pid_t) -> anyhow::Result<()> {
        let unwind_data_path = folder.as_ref().join(format!(
            "{}_{:x}_{:x}.unwind",
            pid, self.avma_range.start, self.avma_range.end
        ));
        self.to_file(unwind_data_path)?;

        Ok(())
    }

    fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        if let Ok(true) = std::fs::exists(path.as_ref()) {
            log::warn!(
                "{} already exists, file will be truncated",
                path.as_ref().display()
            );
            log::warn!("{} {:x?}", self.path, self.avma_range);
        }

        let mut writer = std::fs::File::create(path.as_ref())?;
        bincode::serialize_into(&mut writer, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: You can double-check the values by getting the /proc/<pid>/maps via gdb:
    // ```
    // $ gdb testdata/perf_map/<sample>.bin -ex "break main" -ex "run" -ex "info proc mappings" -ex "continue" -ex "quit" -batch
    // Start Addr         End Addr           Size               Offset             Perms File
    // 0x0000555555554000 0x00005555555a2000 0x4e000            0x0                r--p  /runner/testdata/perf_map/divan_sleep_benches.bin
    // 0x00005555555a2000 0x0000555555692000 0xf0000            0x4d000            r-xp  /runner/testdata/perf_map/divan_sleep_benches.bin
    // 0x0000555555692000 0x000055555569d000 0xb000             0x13c000           r--p  /runner/testdata/perf_map/divan_sleep_benches.bin
    // 0x000055555569d000 0x000055555569f000 0x2000             0x146000           rw-p  /runner/testdata/perf_map/divan_sleep_benches.bin
    // 0x00007ffff7c00000 0x00007ffff7c28000 0x28000            0x0                r--p  /nix/store/g8zyryr9cr6540xsyg4avqkwgxpnwj2a-glibc-2.40-66/lib/libc.so.6
    // 0x00007ffff7c28000 0x00007ffff7d9e000 0x176000           0x28000            r-xp  /nix/store/g8zyryr9cr6540xsyg4avqkwgxpnwj2a-glibc-2.40-66/lib/libc.so.6
    // 0x00007ffff7d9e000 0x00007ffff7df4000 0x56000            0x19e000           r--p  /nix/store/g8zyryr9cr6540xsyg4avqkwgxpnwj2a-glibc-2.40-66/lib/libc.so.6
    // 0x00007ffff7df4000 0x00007ffff7df8000 0x4000             0x1f3000           r--p  /nix/store/g8zyryr9cr6540xsyg4avqkwgxpnwj2a-glibc-2.40-66/lib/libc.so.6
    // 0x00007ffff7df8000 0x00007ffff7dfa000 0x2000             0x1f7000           rw-p  /nix/store/g8zyryr9cr6540xsyg4avqkwgxpnwj2a-glibc-2.40-66/lib/libc.so.6
    // 0x00007ffff7dfa000 0x00007ffff7e07000 0xd000             0x0                rw-p
    // 0x00007ffff7f8a000 0x00007ffff7f8d000 0x3000             0x0                rw-p
    // ...
    // ```

    #[test]
    fn test_golang_unwind_data() {
        const MODULE_PATH: &str = "testdata/perf_map/go_fib.bin";

        let (start_addr, end_addr) = (0x0000000000402000_u64, 0x000000000050f000_u64);
        let size: u64 = end_addr - start_addr;

        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            0x2000,
            start_addr,
            size,
            None
        ));
    }

    #[test]
    fn test_cpp_unwind_data() {
        const MODULE_PATH: &str = "testdata/perf_map/cpp_my_benchmark.bin";

        let (start_addr, end_addr) = (0x0000000000400000_u64, 0x0000000000459000_u64);
        let size: u64 = end_addr - start_addr;

        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            0x0,
            start_addr,
            size,
            None
        ));
    }

    #[test]
    fn test_rust_divan_unwind_data() {
        const MODULE_PATH: &str = "testdata/perf_map/divan_sleep_benches.bin";

        let (start_addr, end_addr) = (0x00005555555a2000_u64, 0x0000555555692000_u64);
        let size: u64 = end_addr - start_addr;

        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            0x4d000,
            start_addr,
            size,
            None
        ));
    }
}
