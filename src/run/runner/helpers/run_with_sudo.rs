use crate::{local_logger::suspend_progress_bar, prelude::*};
use std::{
    io::IsTerminal,
    process::{Command, Stdio},
};

/// Validate sudo access, prompting the user for their password if necessary
fn validate_sudo_access() -> Result<()> {
    let needs_password = IsTerminal::is_terminal(&std::io::stdout())
        && Command::new("sudo")
            .arg("--non-interactive") // Fail if password is required
            .arg("true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| !status.success())
            .unwrap_or(true);

    if needs_password {
        suspend_progress_bar(|| {
            info!(
                "Sudo privileges are required to continue. Please enter your password if prompted."
            );

            // Validate and cache sudo credentials
            let auth_status = Command::new("sudo")
                .arg("--validate") // Validate and extend the timeout
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|_| anyhow!("Failed to authenticate with sudo"))?;

            if !auth_status.success() {
                bail!("Failed to authenticate with sudo");
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Creates the base sudo command after validating sudo access
pub fn validated_sudo_command() -> Result<Command> {
    validate_sudo_access()?;
    let mut cmd = Command::new("sudo");
    // Password prompt should not appear here since it has already been validated
    cmd.arg("--non-interactive");
    Ok(cmd)
}

/// Run a command with sudo after validating sudo access
pub fn run_with_sudo(command_args: &[&str]) -> Result<()> {
    let command_str = command_args.join(" ");
    debug!("Running command with sudo: {command_str}");
    let output = validated_sudo_command()?
        .args(command_args)
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| anyhow!("Failed to execute command with sudo: {command_str}"))?;

    if !output.status.success() {
        info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Failed to execute command with sudo: {command_str}");
    }

    Ok(())
}
