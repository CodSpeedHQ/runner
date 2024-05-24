use crate::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

lazy_static! {
    static ref PERF_MAP_REGEX: Regex = Regex::new(r"perf-(\d+)\.map").unwrap();
}

pub fn harvest_perf_maps(profile_folder: &Path) -> Result<()> {
    // Get profile files (files with .out extension)
    let profile_files = fs::read_dir(profile_folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().unwrap_or_default() == "out")
        .collect_vec();

    let pids = profile_files
        .iter()
        .filter_map(|path| path.file_stem())
        .map(|pid| pid.to_str().unwrap())
        .collect::<HashSet<_>>();

    let perf_map_files = fs::read_dir("/tmp")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| PERF_MAP_REGEX.captures(name))
                .and_then(|captures| captures.get(1))
                .map(|pid| pids.contains(pid.as_str()))
                .unwrap_or(false)
        });

    for perf_map_file in perf_map_files {
        let source_path = perf_map_file.clone();
        let dest_path = profile_folder.join(perf_map_file.file_name().unwrap());
        fs::copy(source_path, dest_path).map_err(|e| {
            anyhow!(
                "Failed to copy perf map file: {} to {}: {}",
                perf_map_file.display(),
                profile_folder.display(),
                e
            )
        })?;
    }

    Ok(())
}
