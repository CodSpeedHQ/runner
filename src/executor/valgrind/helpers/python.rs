use std::process::Command;

/// Checks if the Python interpreter supports free-threaded mode.
/// Returns true if Python is free-threaded (GIL disabled), false otherwise.
pub fn is_free_threaded_python() -> bool {
    // Use sysconfig.get_config_var("Py_GIL_DISABLED") as recommended by Python docs at https://docs.python.org/3/howto/free-threading-python.html#identifying-free-threaded-python
    let output = Command::new("python")
        .args([
            "-c",
            "import sysconfig; print(sysconfig.get_config_var('Py_GIL_DISABLED') or 0)",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim() == "1"
        }
        _ => false, // If Python is not available or command fails, assume not free-threaded
    }
}
