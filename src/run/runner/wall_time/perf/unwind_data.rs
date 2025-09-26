//! WARNING: This file has to be in sync with perf-parser!

use crate::run::runner::wall_time::perf::elf_helper;
use anyhow::{Context, bail};
use debugid::CodeId;
use libc::pid_t;
use object::Object;
use object::ObjectSection;
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
        runtime_file_offset: u64,
        runtime_start_addr: u64,
        runtime_end_addr: u64,
        build_id: Option<&[u8]>,
    ) -> anyhow::Result<Self> {
        let avma_range = runtime_start_addr..runtime_end_addr;

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

        let base_avma = elf_helper::compute_base_avma(
            runtime_start_addr,
            runtime_end_addr,
            runtime_file_offset,
            &file,
        )?;
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

    macro_rules! assert_elf_load_bias {
        ($start_addr:expr, $end_addr:expr, $file_offset:expr, $module_path:expr, $expected_load_bias:expr) => {
            let expected_load_bias = $expected_load_bias as u64;

            let file_data = std::fs::read($module_path).expect("Failed to read test binary");
            let object = object::File::parse(&file_data[..]).expect("Failed to parse test binary");
            let load_bias =
                elf_helper::compute_load_bias($start_addr, $end_addr, $file_offset, &object)
                    .unwrap();
            println!("Load bias for {}: 0x{:x}", $module_path, load_bias);
            assert_eq!(
                load_bias, expected_load_bias,
                "Invalid load bias: {:x} != {:x}",
                load_bias, expected_load_bias
            );
        };
    }

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

        let (start_addr, end_addr, file_offset) =
            (0x0000000000402000_u64, 0x000000000050f000_u64, 0x2000);
        assert_elf_load_bias!(start_addr, end_addr, file_offset, MODULE_PATH, 0x0);
        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            file_offset,
            start_addr,
            end_addr,
            None
        ));
    }

    #[test]
    fn test_cpp_unwind_data() {
        // gdb testdata/perf_map/cpp_my_benchmark.bin -ex "break main" -ex "run" -ex "info proc mappings" -ex "continue" -ex "quit" -batch
        // Start Addr         End Addr           Size               Offset             Perms File
        // 0x0000000000400000 0x0000000000459000 0x59000            0x0                r-xp  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/cpp_my_benchmark.bin
        // 0x000000000045a000 0x000000000045b000 0x1000             0x59000            r--p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/cpp_my_benchmark.bin
        // 0x000000000045b000 0x000000000045c000 0x1000             0x5a000            rw-p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/cpp_my_benchmark.bin
        const MODULE_PATH: &str = "testdata/perf_map/cpp_my_benchmark.bin";

        let (start_addr, end_addr, file_offset) =
            (0x0000000000400000_u64, 0x0000000000459000_u64, 0x0);
        assert_elf_load_bias!(start_addr, end_addr, file_offset, MODULE_PATH, 0x0);

        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            file_offset,
            start_addr,
            end_addr,
            None,
        ));
    }

    #[test]
    fn test_rust_divan_unwind_data() {
        const MODULE_PATH: &str = "testdata/perf_map/divan_sleep_benches.bin";

        let (start_addr, end_addr, file_offset) =
            (0x00005555555a2000_u64, 0x0000555555692000_u64, 0x4d000);
        assert_elf_load_bias!(
            start_addr,
            end_addr,
            file_offset,
            MODULE_PATH,
            0x555555554000
        );
        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            file_offset,
            start_addr,
            end_addr,
            None
        ));
    }

    #[test]
    fn test_the_algorithms_unwind_data() {
        // $ gdb testdata/perf_map/the_algorithms.bin -ex "break main" -ex "run" -ex "info proc mappings" -ex "continue" -ex "quit" -batch
        // Start Addr         End Addr           Size               Offset             Perms File
        // 0x0000555555554000 0x00005555555a7000 0x53000            0x0                r--p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/the_algorithms.bin
        // 0x00005555555a7000 0x00005555556b0000 0x109000           0x52000            r-xp  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/the_algorithms.bin
        // 0x00005555556b0000 0x00005555556bc000 0xc000             0x15a000           r--p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/the_algorithms.bin
        // 0x00005555556bc000 0x00005555556bf000 0x3000             0x165000           rw-p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/the_algorithms.bin

        const MODULE_PATH: &str = "testdata/perf_map/the_algorithms.bin";

        let (start_addr, end_addr, file_offset) = (0x00005555555a7000, 0x00005555556b0000, 0x52000);
        assert_elf_load_bias!(
            start_addr,
            end_addr,
            file_offset,
            MODULE_PATH,
            0x555555554000
        );
        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            file_offset,
            start_addr,
            end_addr,
            None
        ));
    }

    #[test]
    fn test_ruff_unwind_data() {
        // gdb testdata/perf_map/ty_walltime -ex "break main" -ex "run" -ex "info proc mappings" -ex "continue" -ex "quit" -batch
        // 0x0000555555554000 0x0000555555e6d000 0x919000           0x0                r--p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/ty_walltime
        // 0x0000555555e6d000 0x0000555556813000 0x9a6000           0x918000           r-xp  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/ty_walltime
        // 0x0000555556813000 0x00005555568a8000 0x95000            0x12bd000          r--p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/ty_walltime
        // 0x00005555568a8000 0x00005555568ac000 0x4000             0x1351000          rw-p  /home/not-matthias/Documents/work/wgit/runner/testdata/perf_map/ty_walltime
        // 0x00005555568ac000 0x00005555568ad000 0x1000             0x0                rw-p

        const MODULE_PATH: &str = "testdata/perf_map/ty_walltime";
        let (start_addr, end_addr, file_offset) =
            (0x0000555555e6d000_u64, 0x0000555556813000_u64, 0x918000);
        assert_elf_load_bias!(
            start_addr,
            end_addr,
            file_offset,
            MODULE_PATH,
            0x555555554000
        );

        insta::assert_debug_snapshot!(UnwindData::new(
            MODULE_PATH.as_bytes(),
            file_offset,
            start_addr,
            end_addr,
            None
        ));
    }
}
