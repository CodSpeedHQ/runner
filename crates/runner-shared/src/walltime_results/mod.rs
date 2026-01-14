mod interfaces;
mod stats;

pub use interfaces::*;

impl WalltimeResults {
    pub fn from_benchmarks(benchmarks: Vec<WalltimeBenchmark>) -> anyhow::Result<Self> {
        Ok(WalltimeResults {
            instrument: Instrument {
                type_: "walltime".to_string(),
            },
            creator: Creator {
                // TODO(COD-1736): Stop impersonating codspeed-rust 🥸
                name: "codspeed-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                pid: std::process::id(),
            },
            benchmarks,
        })
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, profile_folder: P) -> anyhow::Result<()> {
        let results_path = {
            let results_dir = profile_folder.as_ref().join("results");
            std::fs::create_dir_all(&results_dir).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create results directory: {}: {}",
                    results_dir.display(),
                    e
                )
            })?;

            results_dir.join(format!("{}.json", self.creator.pid))
        };

        let file = std::fs::File::create(&results_path).map_err(|e| {
            anyhow::anyhow!("Failed to create file: {}: {}", results_path.display(), e)
        })?;
        serde_json::to_writer_pretty(file, &self).map_err(|e| {
            anyhow::anyhow!(
                "Failed to write JSON to file: {}: {}",
                results_path.display(),
                e
            )
        })?;
        Ok(())
    }
}
