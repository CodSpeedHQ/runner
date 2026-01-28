//! Named like this because `use` is a keyword

use crate::prelude::*;
use crate::runner_mode::RunnerMode;
use clap::Args;

#[derive(Debug, Args)]
pub struct UseArgs {
    /// Set the CodSpeed runner mode for this shell session. If not provided, the current mode will
    /// be displayed.
    pub mode: RunnerMode,
}

pub fn run(args: UseArgs) -> Result<()> {
    crate::runner_mode::register_shell_session_mode(&args.mode)?;
    debug!(
        "Registered codspeed use mode '{:?}' for this shell session (parent PID)",
        args.mode
    );
    Ok(())
}
