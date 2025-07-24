use crate::prelude::*;
use std::path::Path;

mod parser;
mod walltime_results;

pub fn collect_walltime_results(stdout: &str, dst_dir: &Path) -> Result<()> {
    let benchmarks = parser::BenchmarkData::process_raw_results(parser::RawOutput::parse(stdout)?)
        .into_iter()
        .map(|result| {
            let uri = format!("{}::{}", result.package, result.name);
            walltime_results::WalltimeBenchmark::from_runtime_data(
                result.name,
                uri,
                result.iters.into_iter().map(|i| i as u128).collect(),
                result.times.into_iter().map(|t| t as u128).collect(),
                None,
            )
        })
        .collect::<Vec<_>>();
    debug!("Parsed {} benchmarks", benchmarks.len());

    let pid = std::process::id();
    let creator = walltime_results::Creator {
        name: "runner".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        pid,
    };
    let results = walltime_results::WalltimeResults::new(benchmarks, creator)?;

    let mut file = std::fs::File::create(dst_dir.join(format!("{pid}.json")))?;
    serde_json::to_writer_pretty(&mut file, &results)?;

    Ok(())
}
