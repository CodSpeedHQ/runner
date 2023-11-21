use crate::prelude::*;
use std::process::Command;

fn get_python_objects() -> Vec<String> {
    let output = Command::new("python")
        .arg("-c")
        .arg("import sysconfig; print('/'.join(sysconfig.get_config_vars('LIBDIR', 'INSTSONAME')))")
        .output();

    if output.is_err() {
        debug!("Failed to get python shared objects: {:?}", output.err());
        return vec![];
    }
    let output = output.unwrap();
    if !output.status.success() {
        debug!("Failed to get python shared objects: {:?}", output.stderr);
        return vec![];
    }

    let so_output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    vec![so_output]
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
        debug!("Failed to get node shared objects: {:?}", output.stderr);
        return vec![];
    }
    let so_output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    vec![so_output]
}

pub fn get_objects_path_to_ignore() -> Vec<String> {
    let mut objects_path_to_ignore = vec![];
    objects_path_to_ignore.extend(get_python_objects());
    objects_path_to_ignore.extend(get_node_objects());
    debug!("objects_path_to_ignore: {:?}", objects_path_to_ignore);
    objects_path_to_ignore
}
