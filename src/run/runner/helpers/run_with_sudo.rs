use crate::prelude::*;
use std::process::{Command, Stdio};

/// Run a command with sudo if available
pub fn run_with_sudo(command_args: &[&str]) -> Result<()> {
    let use_sudo = Command::new("sudo")
        // `sudo true` will fail if sudo does not exist or the current user does not have sudo privileges
        .arg("true")
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let mut command_args: Vec<&str> = command_args.into();
    if use_sudo {
        command_args.insert(0, "sudo");
    }

    debug!("Running command: {}", command_args.join(" "));
    let output = Command::new(command_args[0])
        .args(&command_args[1..])
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| anyhow!("Failed to execute command: {}", command_args.join(" ")))?;

    if !output.status.success() {
        info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Failed to execute command: {}", command_args.join(" "));
    }

    Ok(())
}
