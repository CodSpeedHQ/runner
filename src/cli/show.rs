use crate::prelude::*;

pub fn run() -> Result<()> {
    let shell_session_mode = crate::runner_mode::load_shell_session_mode()?;

    if let Some(mode) = shell_session_mode {
        info!("{mode:?}");
    } else {
        info!("No mode set for this shell session");
    }

    Ok(())
}
