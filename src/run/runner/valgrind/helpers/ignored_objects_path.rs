use crate::prelude::*;
use std::{path::PathBuf, process::Command};

fn find_uv_python_paths() -> anyhow::Result<Vec<String>> {
    let output = Command::new("uv")
        .args([
            "python",
            "list",
            "--only-installed",
            "--output-format",
            "json",
        ])
        // IMPORTANT: Set to the cwd, so that we also find python
        // installations in virtual environments.
        .current_dir(std::env::current_dir()?)
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to get uv python paths: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let json_output = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_output).unwrap_or_default();
    let arr = json
        .as_array()
        .context("Failed to parse uv python paths: not an array")?;
    let paths: Vec<String> = arr
        .iter()
        .filter_map(|obj| obj.get("path"))
        .filter_map(|p| p.as_str())
        .map(|s| s.to_string())
        .collect();
    Ok(paths)
}

fn find_system_python_paths() -> anyhow::Result<Vec<String>> {
    let output = Command::new("which").args(["-a", "python"]).output()?;
    if !output.status.success() {
        bail!(
            "Failed to get system python path: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let paths = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .collect();
    Ok(paths)
}

fn find_python_paths() -> anyhow::Result<Vec<String>> {
    let uv_paths = find_uv_python_paths().unwrap_or_default();
    let system_paths = find_system_python_paths().unwrap_or_default();

    let mut paths = uv_paths;
    paths.extend(system_paths);
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn get_python_objects() -> Vec<String> {
    let mut python_objects = Vec::new();
    for path in find_python_paths().unwrap_or_default() {
        // Get the parent directory of the python binary, then join with lib
        let python_path = PathBuf::from(&path);
        let Some(parent_dir) = python_path.parent() else {
            continue;
        };
        let Some(install_dir) = parent_dir.parent() else {
            continue;
        };

        let lib_dir = install_dir.join("lib");
        let Ok(entries) = std::fs::read_dir(&lib_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if !file_name_str.starts_with("libpython") {
                continue;
            }

            let entry_path = entry.path();
            let Some(full_path) = entry_path.to_str() else {
                continue;
            };
            python_objects.push(full_path.to_string());
        }
    }

    python_objects
}

fn get_node_objects() -> Vec<String> {
    let output = Command::new("node")
        .arg("-e")
        .arg("console.log(process.execPath);")
        .output();

    if output.is_err() {
        debug!("Failed to get node shared objects: {:?}", output.err());
        return vec![];
    }
    let output = output.unwrap();
    if !output.status.success() {
        debug!(
            "Failed to get node shared objects: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return vec![];
    }
    let so_output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    vec![so_output]
}

fn normalize_object_paths(objects_path_to_ignore: &mut [String]) {
    for path in objects_path_to_ignore.iter_mut() {
        let cpath = PathBuf::from(&path).canonicalize();
        if cpath.is_err() {
            debug!("Failed to get normalized shared objects: {:?}", cpath.err());
            continue;
        }
        *path = cpath.unwrap().to_string_lossy().to_string();
    }
}

pub fn get_objects_path_to_ignore() -> Vec<String> {
    let mut objects_path_to_ignore = vec![];
    objects_path_to_ignore.extend(get_node_objects());
    debug!("objects_path_to_ignore before normalization: {objects_path_to_ignore:?}");
    normalize_object_paths(&mut objects_path_to_ignore);
    debug!("objects_path_to_ignore after normalization: {objects_path_to_ignore:?}");

    objects_path_to_ignore.extend(get_python_objects());
    objects_path_to_ignore.extend(find_python_paths().unwrap_or_default());

    objects_path_to_ignore.sort();
    objects_path_to_ignore.dedup();

    objects_path_to_ignore
}
