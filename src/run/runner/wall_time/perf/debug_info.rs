use crate::prelude::*;
use crate::run::runner::wall_time::perf::perf_map::ModuleSymbols;
use libc::pid_t;
use object::{Object, ObjectSection};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::Path};

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    pub addr: u64,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

impl Debug for DebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let location = match (&self.file, self.line) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.clone(),
            _ => String::from("<unknown>"),
        };
        let name = self.name.as_deref().unwrap_or("<no symbol>");
        write!(
            f,
            "DebugInfo {{ addr: {:x}, size: {:x}, name: {}, location: {} }}",
            self.addr, self.size, name, location
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDebugInfo {
    load_bias: u64,
    debug_infos: Vec<DebugInfo>,
}

impl ModuleDebugInfo {
    /// Create debug info from existing symbols by looking up file/line in DWARF
    pub fn from_symbols<P: AsRef<Path>>(path: P, symbols: &ModuleSymbols) -> anyhow::Result<Self> {
        let content = std::fs::read(path.as_ref())?;
        let object = object::File::parse(&*content)?;

        let ctx = Self::create_dwarf_context(&object).context("Failed to create DWARF context")?;
        let load_bias = symbols.load_bias();
        let debug_infos = symbols
            .symbols()
            .iter()
            .filter_map(|symbol| {
                let (file, line) = match ctx.find_location(symbol.addr) {
                    Ok(Some(location)) => {
                        let file = location.file.map(|f| f.to_string())?;
                        (file, location.line)
                    }
                    _ => return None,
                };

                Some(DebugInfo {
                    addr: symbol.addr,
                    size: symbol.size,
                    name: Some(symbol.name.clone()),
                    file: Some(file),
                    line,
                })
            })
            .collect();

        Ok(Self {
            load_bias,
            debug_infos,
        })
    }

    fn create_dwarf_context(
        object: &object::File,
    ) -> anyhow::Result<addr2line::Context<gimli::EndianRcSlice<gimli::RunTimeEndian>>> {
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        type EndianRcSlice = gimli::EndianRcSlice<gimli::RunTimeEndian>;
        let load_section = |id: gimli::SectionId| -> Result<EndianRcSlice, gimli::Error> {
            let data = object
                .section_by_name(id.name())
                .and_then(|s| s.uncompressed_data().ok())
                .unwrap_or(std::borrow::Cow::Borrowed(&[]));
            Ok(gimli::EndianRcSlice::new(
                std::rc::Rc::from(data.as_ref()),
                endian,
            ))
        };

        let dwarf = gimli::Dwarf::load(load_section)?;
        addr2line::Context::from_dwarf(dwarf).map_err(Into::into)
    }
}

/// Represents all the modules inside a process and their debug info.
pub struct ProcessDebugInfo {
    pid: pid_t,
    modules: Vec<ModuleDebugInfo>,
}

impl ProcessDebugInfo {
    pub fn new(
        pid: pid_t,
        process_symbols: &crate::run::runner::wall_time::perf::perf_map::ProcessSymbols,
    ) -> Self {
        let mut modules = Vec::new();
        for (path, module_symbols) in process_symbols.modules_with_symbols() {
            match ModuleDebugInfo::from_symbols(path, module_symbols) {
                Ok(module_debug_info) => {
                    modules.push(module_debug_info);
                }
                Err(error) => {
                    debug!("Failed to load debug info for module {path:?}: {error}");
                }
            }
        }

        Self { pid, modules }
    }

    pub fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        if self.modules.is_empty() {
            return Ok(());
        }

        let debug_info_path = folder
            .as_ref()
            .join(format!("perf-{}.debuginfo.json", self.pid));
        let json = serde_json::to_string(&self.modules)?;
        std::fs::write(debug_info_path, json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::runner::wall_time::perf::perf_map::ModuleSymbols;

    #[test]
    fn test_golang_debug_info() {
        let (start_addr, end_addr, file_offset) =
            (0x0000000000402000_u64, 0x000000000050f000_u64, 0x2000);
        let module_symbols = ModuleSymbols::new(
            "testdata/perf_map/go_fib.bin",
            start_addr,
            end_addr,
            file_offset,
        )
        .unwrap();
        let module_debug_info =
            ModuleDebugInfo::from_symbols("testdata/perf_map/go_fib.bin", &module_symbols).unwrap();
        insta::assert_debug_snapshot!(module_debug_info.debug_infos);
    }

    #[test]
    fn test_cpp_debug_info() {
        let (start_addr, end_addr, file_offset) =
            (0x0000000000400000_u64, 0x0000000000459000_u64, 0x0);
        let module_symbols = ModuleSymbols::new(
            "testdata/perf_map/cpp_my_benchmark.bin",
            start_addr,
            end_addr,
            file_offset,
        )
        .unwrap();
        let mut module_debug_info = ModuleDebugInfo::from_symbols(
            "testdata/perf_map/cpp_my_benchmark.bin",
            &module_symbols,
        )
        .unwrap();

        module_debug_info.debug_infos.sort_by_key(|d| d.addr);

        insta::assert_debug_snapshot!(module_debug_info.debug_infos);
    }

    #[test]
    fn test_rust_divan_debug_info() {
        const MODULE_PATH: &str = "testdata/perf_map/divan_sleep_benches.bin";

        let module_symbols =
            ModuleSymbols::new(MODULE_PATH, 0x00005555555a2000, 0x0000555555692000, 0x4d000)
                .unwrap();
        let module_debug_info =
            ModuleDebugInfo::from_symbols(MODULE_PATH, &module_symbols).unwrap();
        insta::assert_debug_snapshot!(module_debug_info.debug_infos);
    }

    #[test]
    fn test_the_algorithms_debug_info() {
        const MODULE_PATH: &str = "testdata/perf_map/the_algorithms.bin";

        let module_symbols = ModuleSymbols::new(
            MODULE_PATH,
            0x00005573e59fe000,
            0x00005573e5b07000,
            0x00052000,
        )
        .unwrap();
        let module_debug_info =
            ModuleDebugInfo::from_symbols(MODULE_PATH, &module_symbols).unwrap();
        insta::assert_debug_snapshot!(module_debug_info.debug_infos);
    }

    #[test]
    fn test_ruff_debug_info() {
        const MODULE_PATH: &str = "testdata/perf_map/ty_walltime";

        let (start_addr, end_addr, file_offset) =
            (0x0000555555e6d000_u64, 0x0000555556813000_u64, 0x918000);
        let module_symbols =
            ModuleSymbols::new(MODULE_PATH, start_addr, end_addr, file_offset).unwrap();
        let module_debug_info =
            ModuleDebugInfo::from_symbols(MODULE_PATH, &module_symbols).unwrap();
        insta::assert_debug_snapshot!(module_debug_info.debug_infos);
    }
}
