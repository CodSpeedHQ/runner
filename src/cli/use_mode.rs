//! Named like this because `use` is a keyword

use crate::prelude::*;
use crate::runner_mode::RunnerMode;
use clap::Args;

#[derive(Debug, Args)]
pub struct UseArgs {
    /// Set the CodSpeed runner mode for this shell session. If not provided, the current mode will
    /// be displayed.
    pub mode: Option<RunnerMode>,
}

pub fn run(args: UseArgs) -> Result<()> {
    if let Some(mode) = &args.mode {
        crate::runner_mode::register_shell_session_mode(mode)?;
        debug!(
            "Registered codspeed use mode '{:?}' for this shell session (parent PID)",
            args.mode
        );
    } else {
        let shell_session_mode = crate::runner_mode::load_shell_session_mode()?;

        if let Some(mode) = shell_session_mode {
            info!("{mode:?}");
        } else {
            info!("No mode set for this shell session");
        }
    }
    Ok(())
}
