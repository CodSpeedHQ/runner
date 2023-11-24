use crate::{config::Config, prelude::*};
use std::path::PathBuf;

use super::{
    check_system::check_system,
    helpers::{perf_maps::harvest_perf_maps, profile_folder::create_profile_folder},
    setup::setup,
    valgrind,
};

pub struct RunData {
    pub profile_folder: PathBuf,
}

pub async fn run(config: &Config) -> Result<RunData> {
    if !config.skip_setup {
        let system_info = check_system()?;
        setup(&system_info).await?;
    }
    //TODO: add valgrind version check
    let profile_folder = create_profile_folder()?;
    valgrind::measure(config, &profile_folder)?;
    harvest_perf_maps(&profile_folder)?;
    Ok(RunData { profile_folder })
}
