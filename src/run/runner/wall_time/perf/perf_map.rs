use log::warn;
use object::{Object, ObjectSymbol, ObjectSymbolTable};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct Symbol {
    offset: u64,
    size: u64,
    name: String,
}

#[derive(Debug)]
pub struct ModuleSymbols {
    path: PathBuf,
    pid: u32,

    base_addr: u64,
    symbols: Vec<Symbol>,
}

impl ModuleSymbols {
    pub fn new<P: AsRef<Path>>(pid: u32, path: P, addr: u64) -> Option<Self> {
        let Ok(content) = std::fs::read(path.as_ref()) else {
            return None;
        };

        let object = object::File::parse(&*content).ok()?;
        let symbols = object
            .symbol_table()?
            .symbols()
            .filter_map(|symbol| {
                Some(Symbol {
                    offset: symbol.address(),
                    size: symbol.size(),
                    name: symbol.name().ok()?.to_string(),
                })
            })
            .filter(|symbol| symbol.offset > 0 && symbol.size > 0)
            .collect();

        Some(Self {
            path: path.as_ref().to_path_buf(),
            symbols,
            pid,
            base_addr: addr,
        })
    }

    pub fn merge(&mut self, other: &ModuleSymbols) {
        assert_eq!(other.pid, self.pid);
        assert_eq!(other.path, self.path);
        assert_eq!(self.symbols.len(), other.symbols.len());

        // The symbols are relative to the base address, so we need to find it
        // based on all the sections that are part of the module.
        self.base_addr = core::cmp::min(self.base_addr, other.base_addr);
    }

    fn append_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        for symbol in &self.symbols {
            writeln!(
                file,
                "{:x} {:x} {}",
                self.base_addr + symbol.offset,
                symbol.size,
                symbol.name
            )?;
        }

        Ok(())
    }
}

/// Represents all the modules inside a process and their symbols.
pub struct ProcessSymbols {
    modules: HashMap<PathBuf, ModuleSymbols>,
    pid: u32,
}

impl ProcessSymbols {
    pub fn new(pid: u32) -> Self {
        Self {
            modules: HashMap::new(),
            pid,
        }
    }

    pub fn add_module_symbols(&mut self, symbols: ModuleSymbols) {
        if self.pid != symbols.pid {
            warn!("pid mismatch: {} != {}", self.pid, symbols.pid);
            return;
        }

        self.modules
            .entry(symbols.path.clone())
            .and_modify(|existing| existing.merge(&symbols))
            .or_insert(symbols);
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
