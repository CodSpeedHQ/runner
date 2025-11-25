use anyhow::Context;
use anyhow::Result;
use codspeed::walltime_results::WalltimeBenchmark;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Instrument {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Creator {
    name: String,
    version: String,
    pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalltimeResults {
    creator: Creator,
    instrument: Instrument,
    benchmarks: Vec<WalltimeBenchmark>,
}

impl WalltimeResults {
    pub fn from_benchmarks(benchmarks: Vec<WalltimeBenchmark>) -> Result<Self> {
        Ok(WalltimeResults {
            instrument: Instrument {
                type_: "walltime".to_string(),
            },
            creator: Creator {
                // TODO: Stop impersonating codspeed-rust 🥸
                name: "codspeed-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                pid: std::process::id(),
            },
            benchmarks,
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, profile_folder: P) -> Result<()> {
        let results_path = {
            let results_dir = profile_folder.as_ref().join("results");
            std::fs::create_dir_all(&results_dir).with_context(|| {
                format!(
                    "Failed to create results directory: {}",
                    results_dir.display()
                )
            })?;

            results_dir.join(format!("{}.json", self.creator.pid))
        };

        let file = std::fs::File::create(&results_path)
            .with_context(|| format!("Failed to create file: {}", results_path.display()))?;
        serde_json::to_writer_pretty(file, &self)
            .with_context(|| format!("Failed to write JSON to file: {}", results_path.display()))?;
        Ok(())
    }
}
