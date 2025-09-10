use crate::prelude::*;
use object::{Object, ObjectSegment, ObjectSymbol, ObjectSymbolTable};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Symbol {
    pub addr: u64,
    pub size: u64,
    pub name: String,
}

impl Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol {{ offset: {:x}, size: {:x}, name: {} }}",
            self.addr, self.size, self.name
        )
    }
}

#[derive(Debug, Clone)]
pub struct ModuleSymbols {
    symbols: Vec<Symbol>,
}

impl ModuleSymbols {
    pub fn from_symbols(symbols: Vec<Symbol>) -> Self {
        Self { symbols }
    }

    pub fn new<P: AsRef<Path>>(
        path: P,
        runtime_start_addr: u64,
        runtime_offset: u64,
    ) -> anyhow::Result<Self> {
        let content = std::fs::read(path.as_ref())?;
        let object = object::File::parse(&*content)?;

        let mut symbols = Vec::new();

        if let Some(symbol_table) = object.symbol_table() {
            symbols.extend(symbol_table.symbols().filter_map(|symbol| {
                Some(Symbol {
                    addr: symbol.address(),
                    size: symbol.size(),
                    name: symbol.name().ok()?.to_string(),
                })
            }));
        }

        if let Some(symbol_table) = object.dynamic_symbol_table() {
            symbols.extend(symbol_table.symbols().filter_map(|symbol| {
                Some(Symbol {
                    addr: symbol.address(),
                    size: symbol.size(),
                    name: symbol.name().ok()?.to_string(),
                })
            }));
        }

        symbols.retain(|symbol| symbol.addr > 0 && symbol.size > 0);
        if symbols.is_empty() {
            return Err(anyhow::anyhow!("No symbols found"));
        }

        let load_bias = Self::compute_load_bias(runtime_start_addr, runtime_offset, &object)?;
        for symbol in &mut symbols {
            symbol.addr = symbol.addr.wrapping_add(load_bias);
        }

        Ok(Self { symbols })
    }

    fn compute_load_bias(
        runtime_start_addr: u64,
        runtime_offset: u64,
        object: &object::File,
    ) -> anyhow::Result<u64> {
        // The addresses of symbols read from an ELF file on disk are not their final runtime addresses.
        // This is due to Address Space Layout Randomization (ASLR) and the way the OS loader maps
        // file segments into virtual memory.
        //
        // Step 1: Find the corresponding ELF segment.
        // We must find the `PT_LOAD` segment that corresponds to the executable memory region we found
        // in /proc/<pid>/maps. We do this by comparing the `runtime_offset` against the offset in the file.
        //
        // For example, if we have the following `/proc/<pid>/maps` output:
        // ```
        // 00400000-00402000 r--p 00000000 fe:01 114429641            /runner/testdata/perf_map/go_fib.bin
        // 00402000-0050f000 r-xp 00002000 fe:01 114429641            /runner/testdata/perf_map/go_fib.bin      <-- we find this
        // 0050f000-0064b000 r--p 0010f000 fe:01 114429641            /runner/testdata/perf_map/go_fib.bin
        // 0064b000-0064c000 r--p 0024a000 fe:01 114429641            /runner/testdata/perf_map/go_fib.bin
        // 0064c000-0065e000 rw-p 0024b000 fe:01 114429641            /runner/testdata/perf_map/go_fib.bin
        // 0065e000-00684000 rw-p 00000000 00:00 0
        // ```
        //
        // We'll match the PT_LOAD segment with the same offset (0x2000):
        // ```
        // $ readelf -l testdata/perf_map/go_fib.bin
        // Elf file type is EXEC (Executable file)
        // Entry point 0x402490
        // There are 15 program headers, starting at offset 64
        //
        // Program Headers:
        //   Type           Offset             VirtAddr           PhysAddr
        //   PHDR           0x0000000000000040 0x0000000000400040 0x0000000000400040
        //                  0x0000000000000348 0x0000000000000348  R      0x8
        //   INTERP         0x0000000000000430 0x0000000000400430 0x0000000000400430
        //                  0x0000000000000053 0x0000000000000053  R      0x1
        //   LOAD           0x0000000000000000 0x0000000000400000 0x0000000000400000
        //                  0x0000000000001640 0x0000000000001640  R      0x1000
        //   LOAD           0x0000000000002000 0x0000000000402000 0x0000000000402000        <-- we'll match this
        //                  0x000000000010ceb1 0x000000000010ceb1  R E    0x1000
        // ```
        let load_segment = object
            .segments()
            .find(|segment| {
                // When the kernel loads an ELF file, it maps entire pages (usually 4KB aligned),
                // not just the exact segment boundaries. Here's what happens:
                //
                // **ELF File Structure**:
                // - LOAD segment 1: file offset 0x0      - 0x4d26a  (data/code)
                // - LOAD segment 2: file offset 0x4d26c  - 0x13c4b6 (executable code)
                //
                // **Kernel Memory Mapping**: The kernel rounds down to page boundaries when mapping:
                // - Maps pages starting at offset 0x0     (covers segment 1)
                // - Maps pages starting at offset 0x4d000 (page-aligned, covers segment 2)
                //
                // (the example values are based on the `test_rust_divan_symbols` test)
                let (file_offset, file_size) = segment.file_range();
                runtime_offset >= file_offset && runtime_offset < file_offset + file_size
            })
            .context("Failed to find a matching PT_LOAD segment")?;

        // Step 2: Calculate the "load bias".
        // The bias is the difference between where the segment *actually* is in memory versus where the
        // ELF file *preferred* it to be.
        //
        //   load_bias = runtime_start_addr - segment_preferred_vaddr
        //
        //  - `runtime_start_addr`: The actual base address of this segment in memory (from `/proc/maps`).
        //  - `load_segment.address()`: The preferred virtual address (`p_vaddr`) from the ELF file itself.
        //
        // This single calculation correctly handles both PIE/shared-objects and non-PIE executables:
        //  - For PIE/.so files:   `0x7f... (random) - 0x... (small) = <large_bias>`
        //  - For non-PIE files: `0x402000 (fixed) - 0x402000 (fixed) = 0`
        Ok(runtime_start_addr.wrapping_sub(load_segment.address()))
    }

    pub fn append_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        for symbol in &self.symbols {
            writeln!(file, "{:x} {:x} {}", symbol.addr, symbol.size, symbol.name)?;
        }

        Ok(())
    }
}

/// Represents all the modules inside a process and their symbols.
pub struct ProcessSymbols {
    pid: u32,
    module_mappings: HashMap<PathBuf, Vec<(u64, u64)>>,
    modules: HashMap<PathBuf, ModuleSymbols>,
}

impl ProcessSymbols {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            module_mappings: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    pub fn add_mapping<P: AsRef<Path>>(
        &mut self,
        pid: u32,
        module_path: P,
        start_addr: u64,
        end_addr: u64,
        file_offset: u64,
    ) {
        if self.pid != pid {
            warn!("pid mismatch: {} != {}", self.pid, pid);
            return;
        }

        debug!("Loading module symbols at {start_addr:x}-{end_addr:x} (offset: {file_offset:x})");
        let path = module_path.as_ref().to_path_buf();
        match ModuleSymbols::new(module_path, start_addr, file_offset) {
            Ok(symbol) => {
                self.modules.entry(path.clone()).or_insert(symbol);
            }
            Err(error) => {
                debug!("Failed to load symbols for module {path:?}: {error}");
            }
        }

        self.module_mappings
            .entry(path.clone())
            .or_default()
            .push((start_addr, end_addr));
    }

    pub fn loaded_modules(&self) -> impl Iterator<Item = &PathBuf> {
        self.modules.keys()
    }

    pub fn module_mapping<P: AsRef<std::path::Path>>(
        &self,
        module_path: P,
    ) -> Option<&[(u64, u64)]> {
        self.module_mappings
            .get(module_path.as_ref())
            .map(|bounds| bounds.as_slice())
    }

    pub fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        if self.modules.is_empty() {
            return Ok(());
        }

        let symbols_path = folder.as_ref().join(format!("perf-{}.map", self.pid));
        for module in self.modules.values() {
            module.append_to_file(&symbols_path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golang_symbols() {
        let module_symbols =
            ModuleSymbols::new("testdata/perf_map/go_fib.bin", 0x00402000, 0x00002000).unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }

    #[test]
    fn test_cpp_symbols() {
        const MODULE_PATH: &str = "testdata/perf_map/cpp_my_benchmark.bin";
        let module_symbols = ModuleSymbols::new(MODULE_PATH, 0x00400000, 0x00000000).unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }

    #[test]
    fn test_rust_divan_symbols() {
        const MODULE_PATH: &str = "testdata/perf_map/divan_sleep_benches.bin";

        // Segments in the file:
        // Segment: Segment { address: 0, size: 4d26a }
        // Segment: Segment { address: 4e26c, size: ef24a }
        // Segment: Segment { address: 13e4b8, size: ab48 }
        // Segment: Segment { address: 1499b0, size: 11a5 }
        //
        // Segments in memory:
        // 0x0000555555554000 0x00005555555a2000 0x4e000            0x0                r--p
        // 0x00005555555a2000 0x0000555555692000 0xf0000            0x4d000            r-xp         <--
        // 0x0000555555692000 0x000055555569d000 0xb000             0x13c000           r--p
        // 0x000055555569d000 0x000055555569f000 0x2000             0x146000           rw-p
        //
        let module_symbols = ModuleSymbols::new(MODULE_PATH, 0x00005555555a2000, 0x4d000).unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }
}
