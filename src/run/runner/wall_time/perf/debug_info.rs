use crate::prelude::*;
use crate::run::runner::wall_time::perf::perf_map::ModuleSymbols;
use addr2line::gimli;
use object::{Object, ObjectSection};
use runner_shared::debug_info::{DebugInfo, ModuleDebugInfo};
use std::path::Path;

type EndianRcSlice = gimli::EndianRcSlice<gimli::RunTimeEndian>;

pub trait ModuleDebugInfoExt {
    fn from_symbols<P: AsRef<Path>>(path: P, symbols: &ModuleSymbols) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn create_dwarf_context(
        object: &object::File,
    ) -> anyhow::Result<addr2line::Context<EndianRcSlice>> {
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        let load_section = |id: gimli::SectionId| -> Result<EndianRcSlice, gimli::Error> {
            let data = object
                .section_by_name(id.name())
                .and_then(|s| s.uncompressed_data().ok())
                .unwrap_or(std::borrow::Cow::Borrowed(&[]));
            Ok(EndianRcSlice::new(std::rc::Rc::from(data.as_ref()), endian))
        };

        let dwarf = gimli::Dwarf::load(load_section)?;
        addr2line::Context::from_dwarf(dwarf).map_err(Into::into)
    }
}

impl ModuleDebugInfoExt for ModuleDebugInfo {
    /// Create debug info from existing symbols by looking up file/line in DWARF
    fn from_symbols<P: AsRef<Path>>(path: P, symbols: &ModuleSymbols) -> anyhow::Result<Self> {
        let content = std::fs::read(path.as_ref())?;
        let object = object::File::parse(&*content)?;

        let ctx = Self::create_dwarf_context(&object).context("Failed to create DWARF context")?;
        let load_bias = symbols.load_bias();
        let (mut min_addr, mut max_addr) = (None, None);
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

                min_addr = Some(min_addr.map_or(symbol.addr, |addr: u64| addr.min(symbol.addr)));
                max_addr = Some(max_addr.map_or(symbol.addr + symbol.size, |addr: u64| {
                    addr.max(symbol.addr + symbol.size)
                }));

                Some(DebugInfo {
                    addr: symbol.addr,
                    size: symbol.size,
                    name: symbol.name.clone(),
                    file,
                    line,
                })
            })
            // Sort by address, to allow binary search lookups in backend
            .sorted_by_key(|d| d.addr)
            .collect();

        let (Some(min_addr), Some(max_addr)) = (min_addr, max_addr) else {
            anyhow::bail!("No debug info could be extracted from module");
        };

        Ok(ModuleDebugInfo {
            object_path: path.as_ref().to_string_lossy().to_string(),
            load_bias,
            addr_bounds: (min_addr, max_addr),
            debug_infos,
        })
    }
}

/// Represents all the modules inside a process and their debug info.
pub struct ProcessDebugInfo {
    modules: Vec<ModuleDebugInfo>,
}

impl ProcessDebugInfo {
    pub fn new(
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

        Self { modules }
    }

    /// Returns the debug info modules for this process
    pub fn modules(self) -> Vec<ModuleDebugInfo> {
        self.modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
