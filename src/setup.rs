use crate::prelude::*;
use crate::run::check_system::SystemInfo;
use crate::run::runner::get_all_executors;

pub async fn setup() -> Result<()> {
    let system_info = SystemInfo::new()?;
    let executors = get_all_executors();
    start_group!("Setting up the environment for all executors");
    for executor in executors {
        info!(
            "Setting up the environment for the executor: {}",
            executor.name().to_string()
        );
        executor.setup(&system_info).await?;
    }
    info!("Environment setup completed");
    end_group!();
    Ok(())
}
