use crate::{local_logger::suspend_progress_bar, prelude::*};
use std::{
    io::IsTerminal,
    process::{Command, Stdio},
};

fn is_root_user() -> bool {
    #[cfg(unix)]
    return nix::unistd::Uid::current().is_root();
    #[cfg(not(unix))]
    return false;
}

fn is_sudo_available() -> bool {
    Command::new("sudo")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

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

/// Build a command wrapped with sudo if possible
pub fn build_command_with_sudo(command_args: &[&str]) -> Result<Command> {
    let command_str = command_args.join(" ");
    if is_root_user() {
        debug!("Running command without sudo: {command_str}");
        let mut c = Command::new(command_args[0]);
        c.args(&command_args[1..]);
        Ok(c)
    } else if is_sudo_available() {
        debug!("Sudo is required for command: {command_str}");
        let mut c = validated_sudo_command()?;
        c.args(command_args);
        Ok(c)
    } else {
        bail!("Sudo is not available to run the command: {command_str}");
    }
}

/// Run a command with sudo after validating sudo access
pub fn run_with_sudo(command_args: &[&str]) -> Result<()> {
    let command_str = command_args.join(" ");
    debug!("Running command with sudo: {command_str}");
    let mut cmd = build_command_with_sudo(command_args)?;
    let output = cmd
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
