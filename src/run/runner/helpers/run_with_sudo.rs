use crate::{
    local_logger::suspend_progress_bar, prelude::*, run::runner::helpers::command::CommandBuilder,
};
use std::{
    ffi::OsStr,
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
            info!("Sudo privileges are required to continue. Please enter your password.");

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

/// Wrap with sudo if not running as root
pub fn wrap_with_sudo(mut cmd_builder: CommandBuilder) -> Result<CommandBuilder> {
    if is_root_user() {
        Ok(cmd_builder)
    } else if is_sudo_available() {
        debug!("Wrapping with sudo: {}", cmd_builder.as_command_line());
        validate_sudo_access()?;
        cmd_builder.wrap(
            "sudo",
            // Password prompt should not appear here since it has already been validated
            ["--non-interactive"],
        );
        Ok(cmd_builder)
    } else {
        bail!(
            "Sudo is not available to run the command: {}",
            cmd_builder.as_command_line()
        );
    }
}

/// Run a command with sudo after validating sudo access
pub fn run_with_sudo<S, I, T>(program: S, argv: I) -> Result<()>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut builder = CommandBuilder::new(program);
    builder.args(argv);
    debug!("Running command with sudo: {}", builder.as_command_line());
    let mut cmd = wrap_with_sudo(builder)?.build();
    let output = cmd
        .stdout(Stdio::piped())
        .output()
        .map_err(|_| anyhow!("Failed to execute command with sudo: {cmd:?}"))?;

    if !output.status.success() {
        info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Failed to execute command with sudo: {cmd:?}");
    }

    Ok(())
}
