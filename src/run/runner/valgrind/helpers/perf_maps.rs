use crate::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn harvest_perf_maps(profile_folder: &Path) -> Result<()> {
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
        .filter_map(|pid| pid.parse().ok())
        .collect::<HashSet<_>>();

    harvest_perf_maps_for_pids(profile_folder, &pids).await
}

pub async fn harvest_perf_maps_for_pids(profile_folder: &Path, pids: &HashSet<i32>) -> Result<()> {
    let perf_maps = pids
        .iter()
        .map(|pid| format!("perf-{}.map", pid))
        .map(|file_name| {
            (
                PathBuf::from("/tmp").join(&file_name),
                profile_folder.join(&file_name),
            )
        })
        .filter(|(src_path, _)| src_path.exists())
        .collect::<Vec<_>>();
    debug!("Found {} perf maps", perf_maps.len());

    for (src_path, dst_path) in perf_maps {
        fs::copy(&src_path, &dst_path).map_err(|e| {
            anyhow!(
                "Failed to copy perf map file: {:?} to {}: {}",
                src_path.file_name(),
                profile_folder.display(),
                e
            )
        })?;
    }

    Ok(())
}
