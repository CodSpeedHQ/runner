use crate::prelude::*;
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

#[derive(Debug, Clone)]
pub struct ModuleSymbols {
    path: PathBuf,
    symbols: Vec<Symbol>,
}

impl ModuleSymbols {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read(path.as_ref())?;
        let object = object::File::parse(&*content)?;

        let mut symbols = Vec::new();

        if let Some(symbol_table) = object.symbol_table() {
            symbols.extend(symbol_table.symbols().filter_map(|symbol| {
                Some(Symbol {
                    offset: symbol.address(),
                    size: symbol.size(),
                    name: symbol.name().ok()?.to_string(),
                })
            }));
        }

        if let Some(symbol_table) = object.dynamic_symbol_table() {
            symbols.extend(symbol_table.symbols().filter_map(|symbol| {
                Some(Symbol {
                    offset: symbol.address(),
                    size: symbol.size(),
                    name: symbol.name().ok()?.to_string(),
                })
            }));
        }

        symbols.retain(|symbol| symbol.offset > 0 && symbol.size > 0);
        if symbols.is_empty() {
            return Err(anyhow::anyhow!("No symbols found"));
        }

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            symbols,
        })
    }

    fn append_to_file<P: AsRef<Path>>(&self, path: P, base_addr: u64) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        for symbol in &self.symbols {
            writeln!(
                file,
                "{:x} {:x} {}",
                base_addr + symbol.offset,
                symbol.size,
                symbol.name
            )?;
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
    ) {
        if self.pid != pid {
            warn!("pid mismatch: {} != {}", self.pid, pid);
            return;
        }

        let path = module_path.as_ref().to_path_buf();
        match ModuleSymbols::new(module_path) {
            Ok(symbol) => {
                self.modules.entry(path.clone()).or_insert(symbol);
            }
            Err(error) => {
                debug!(
                    "Failed to load symbols for module {}: {}",
                    path.display(),
                    error
                );
            }
        }

        self.module_mappings
            .entry(path.clone())
            .or_default()
            .push((start_addr, end_addr));
    }

    pub fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        if self.modules.is_empty() {
            return Ok(());
        }

        let symbols_path = folder.as_ref().join(format!("perf-{}.map", self.pid));
        for module in self.modules.values() {
            let Some((base_addr, _)) = self
                .module_mappings
                .get(&module.path)
                .and_then(|bounds| bounds.iter().min_by_key(|(start, _)| start))
            else {
                warn!("No bounds found for module: {}", module.path.display());
                continue;
            };
            module.append_to_file(&symbols_path, *base_addr)?;
        }

        Ok(())
    }
}
