use crate::{prelude::*, run::runner::wall_time::perf::elf_helper};
use object::{Object, ObjectSymbol, ObjectSymbolTable};
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
        runtime_end_addr: u64,
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

        let base_avma = elf_helper::compute_base_avma(
            runtime_start_addr,
            runtime_end_addr,
            runtime_offset,
            &object,
        )?;
        for symbol in &mut symbols {
            // Only add the offset if the symbol address is not already an absolute address.
            // This is the case for some Go and CPP binaries.
            if symbol.addr >= base_avma {
                continue;
            }

            symbol.addr = symbol.addr.wrapping_add(base_avma);
        }

        Ok(Self { symbols })
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
        match ModuleSymbols::new(module_path, start_addr, end_addr, file_offset) {
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
        let (start_addr, end_addr, file_offset) =
            (0x0000000000402000_u64, 0x000000000050f000_u64, 0x2000);
        let module_symbols = ModuleSymbols::new(
            "testdata/perf_map/go_fib.bin",
            start_addr,
            end_addr,
            file_offset,
        )
        .unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }

    #[test]
    fn test_cpp_symbols() {
        let (start_addr, end_addr, file_offset) =
            (0x0000000000400000_u64, 0x0000000000459000_u64, 0x0);
        let module_symbols = ModuleSymbols::new(
            "testdata/perf_map/cpp_my_benchmark.bin",
            start_addr,
            end_addr,
            file_offset,
        )
        .unwrap();
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
        let module_symbols =
            ModuleSymbols::new(MODULE_PATH, 0x00005555555a2000, 0x0000555555692000, 0x4d000)
                .unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }

    #[test]
    fn test_the_algorithms_symbols() {
        const MODULE_PATH: &str = "testdata/perf_map/the_algorithms.bin";

        let module_symbols = ModuleSymbols::new(
            MODULE_PATH,
            0x00005573e59fe000,
            0x00005573e5b07000,
            0x00052000,
        )
        .unwrap();
        insta::assert_debug_snapshot!(module_symbols.symbols);
    }
}
