use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::fifo::MarkerType;

#[derive(Serialize, Deserialize)]
pub struct PerfMetadata {
    /// The version of this metadata format.
    pub version: u64,

    /// Name and version of the integration
    pub integration: (String, String),

    /// The URIs of the benchmarks with the timestamps they were executed at.
    pub uri_by_ts: Vec<(u64, String)>,

    /// Modules that should be ignored and removed from the folded trace and callgraph (e.g. python interpreter)
    pub ignored_modules: Vec<(String, u64, u64)>,

    /// Marker for certain regions in the profiling data
    pub markers: Vec<MarkerType>,
}

impl PerfMetadata {
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        serde_json::from_reader(reader).context("Could not parse perf metadata from JSON")
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = std::fs::File::create(path.as_ref().join("perf.metadata"))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}
