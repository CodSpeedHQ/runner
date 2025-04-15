// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH BELOW !!
// !!!!!!!!!!!!!!!!!!!!!!!!
// Has to be in sync with `perf-parser`.
//

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PerfMetadata {
    /// Name and version of the integration
    pub integration: (String, String),

    /// The URIs of the benchmarks in the order they were executed.
    pub bench_order_by_pid: HashMap<u32, Vec<String>>,

    /// Modules that should be ignored and removed from the folded trace and callgraph (e.g. python interpreter)
    pub ignored_modules: Vec<(String, u64, u64)>,
}

impl PerfMetadata {
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = std::fs::File::create(path.as_ref().join("perf.metadata"))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}
