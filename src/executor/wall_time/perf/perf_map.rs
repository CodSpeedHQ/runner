use crate::executor::wall_time::perf::elf_helper;
use crate::prelude::*;
use libc::pid_t;
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
    load_bias: u64,
    symbols: Vec<Symbol>,
}

impl ModuleSymbols {
    pub fn from_symbols(symbols: Vec<Symbol>) -> Self {
        Self {
            symbols,
            load_bias: 0,
        }
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn load_bias(&self) -> u64 {
        self.load_bias
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

        // Filter out
        //  - ARM ELF "mapping symbols" (https://github.com/torvalds/linux/blob/9448598b22c50c8a5bb77a9103e2d49f134c9578/tools/perf/util/symbol-elf.c#L1591C1-L1598C4)
        //  - symbols that have en empty name
        symbols.retain(|symbol| {
            if symbol.name.is_empty() {
                trace!("Filtering out symbol with empty name: {symbol:?}");
                return false;
            }

            // Reject ARM ELF "mapping symbols" as does perf
            let name = symbol.name.as_str();
            if let [b'$', b'a' | b'd' | b't' | b'x', rest @ ..] = name.as_bytes() {
                if rest.is_empty() || rest.starts_with(b".") {
                    trace!("Filtering out ARM ELF mapping symbol: {symbol:?}");
                    return false;
                }
            }

            true
        });

        // Update zero-sized symbols to cover the range until the next symbol
        // This is what perf does
        // https://github.com/torvalds/linux/blob/e538109ac71d801d26776af5f3c54f548296c29c/tools/perf/util/symbol.c#L256
        // A common source for these is inline assembly functions.
        symbols.sort_by_key(|symbol| symbol.addr);
        for i in 0..symbols.len() {
            if symbols[i].size == 0 {
                if i + 1 < symbols.len() {
                    // Set size to the distance to the next symbol
                    symbols[i].size = symbols[i + 1].addr.saturating_sub(symbols[i].addr);
                } else {
                    // Last symbol: round up to next 4KB page boundary and add 4KiB
                    // This matches perf's behavior: roundup(curr->start, 4096) + 4096
                    const PAGE_SIZE: u64 = 4096;
                    let addr = symbols[i].addr;
                    let end_addr = addr.next_multiple_of(PAGE_SIZE) + PAGE_SIZE;
                    symbols[i].size = end_addr.saturating_sub(addr);
                }
            }
        }

        // Filter out any symbols are still zero-sized
        symbols.retain(|symbol| symbol.size > 0);

        if symbols.is_empty() {
            return Err(anyhow::anyhow!("No symbols found"));
        }

        let load_bias = elf_helper::compute_load_bias(
            runtime_start_addr,
            runtime_end_addr,
            runtime_offset,
            &object,
        )?;

        Ok(Self { load_bias, symbols })
    }

    pub fn append_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        for symbol in &self.symbols {
            writeln!(
                file,
                "{:x} {:x} {}",
                symbol.addr.wrapping_add(self.load_bias),
                symbol.size,
                symbol.name
            )?;
        }

        Ok(())
    }
}

/// Represents all the modules inside a process and their symbols.
pub struct ProcessSymbols {
    pid: pid_t,
    module_mappings: HashMap<PathBuf, Vec<(u64, u64)>>,
    modules: HashMap<PathBuf, ModuleSymbols>,
}

impl ProcessSymbols {
    pub fn new(pid: pid_t) -> Self {
        Self {
            pid,
            module_mappings: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    pub fn add_mapping<P: AsRef<Path>>(
        &mut self,
        pid: pid_t,
        module_path: P,
        start_addr: u64,
        end_addr: u64,
        file_offset: u64,
    ) {
        if self.pid != pid {
            warn!("pid mismatch: {} != {}", self.pid, pid);
            return;
        }

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

    pub fn modules_with_symbols(&self) -> impl Iterator<Item = (&PathBuf, &ModuleSymbols)> {
        self.modules.iter()
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
        insta::assert_debug_snapshot!(module_symbols);
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
        insta::assert_debug_snapshot!(module_symbols);
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
        insta::assert_debug_snapshot!(module_symbols);
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
        insta::assert_debug_snapshot!(module_symbols);
    }

    #[test]
    fn test_ruff_symbols() {
        const MODULE_PATH: &str = "testdata/perf_map/ty_walltime";

        let (start_addr, end_addr, file_offset) =
            (0x0000555555e6d000_u64, 0x0000555556813000_u64, 0x918000);
        let module_symbols =
            ModuleSymbols::new(MODULE_PATH, start_addr, end_addr, file_offset).unwrap();
        insta::assert_debug_snapshot!(module_symbols);
    }
}
