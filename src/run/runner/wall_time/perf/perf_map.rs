use linux_perf_data::{
    linux_perf_event_reader::{EventRecord, Mmap2Record, MmapRecord},
    PerfFileReader, PerfFileRecord,
};
use object::{Object, ObjectSymbol, ObjectSymbolTable};
use std::{collections::HashMap, io::Write, path::Path};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct Symbol {
    offset: u64,
    size: u64,
    name: String,
}

#[derive(Debug)]
pub struct ModuleSymbols {
    #[allow(unused)]
    path: String,
    symbols: Vec<Symbol>,
    pid: i32,

    start_addr: u64,
    end_addr: u64,
}

impl ModuleSymbols {
    pub fn new(pid: i32, path: &str, addr: u64, size: u64) -> Option<Self> {
        let Ok(content) = std::fs::read(path) else {
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
            path: path.to_string(),
            symbols,
            pid,
            start_addr: addr,
            end_addr: addr + size,
        })
    }

    /// Merge all of the passed symbols into one module.
    pub fn merge(&mut self, other: &ModuleSymbols) {
        assert_eq!(other.pid, self.pid);

        self.start_addr = core::cmp::min(self.start_addr, other.start_addr);
        self.end_addr = core::cmp::min(self.end_addr, other.end_addr);

        self.symbols.extend(other.symbols.iter().cloned());
        self.symbols.dedup();
    }

    pub fn from_mmap(event: &MmapRecord) -> Option<Self> {
        let path_slice = event.path.as_slice();
        let path = core::str::from_utf8(path_slice.as_ref()).unwrap();

        if !event.is_executable {
            log::debug!("Skipping non-executable record: {}", path);
            return None;
        }

        Self::new(event.pid, path, event.address, event.length)
    }

    pub fn from_mmap2(event: &Mmap2Record) -> Option<Self> {
        let path_slice = event.path.as_slice();
        let path = core::str::from_utf8(path_slice.as_ref()).unwrap();

        Self::new(event.pid, path, event.address, event.length)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        for symbol in &self.symbols {
            writeln!(
                file,
                "{:x} {:x} {}",
                self.start_addr + symbol.offset,
                symbol.size,
                symbol.name
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SyntheticPerfMap {
    module_by_pid: HashMap<i32, ModuleSymbols>,
}

impl SyntheticPerfMap {
    pub fn from_perf_file<P: AsRef<Path>>(path: P) -> Self {
        let file = std::fs::File::open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        let PerfFileReader {
            mut perf_file,
            mut record_iter,
        } = PerfFileReader::parse_file(reader).unwrap();

        let mut module_by_pid = HashMap::<i32, ModuleSymbols>::new();
        while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
            let PerfFileRecord::EventRecord { record, .. } = record else {
                continue;
            };

            let Ok(parsed_record) = record.parse() else {
                continue;
            };

            match parsed_record {
                EventRecord::Mmap(event) => {
                    if let Some(module) = ModuleSymbols::from_mmap(&event) {
                        module_by_pid
                            .entry(event.pid)
                            .and_modify(|existing| existing.merge(&module))
                            .or_insert(module);
                    }
                }
                EventRecord::Mmap2(event) => {
                    if let Some(module) = ModuleSymbols::from_mmap2(&event) {
                        module_by_pid
                            .entry(event.pid)
                            .and_modify(|existing| existing.merge(&module))
                            .or_insert(module);
                    }
                }
                _ => {}
            }
        }

        SyntheticPerfMap { module_by_pid }
    }

    pub fn save_to<P: AsRef<Path>>(&self, folder: P) -> Result<(), std::io::Error> {
        for (pid, module) in &self.module_by_pid {
            let path = folder.as_ref().join(format!("perf-{}.map", pid));
            module.to_file(path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_synthetic_perf_map() {
        let module = ModuleSymbols::new(42, "/usr/local/bin/valgrind", 0x100_000, 0x1000).unwrap();
        assert_eq!(module.symbols.len(), 0, "{:x?}", module);
    }
}
