use crate::{config::Config, instruments::mongo_tracer::MongoTracer, prelude::*};
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
        start_group!("Prepare the environment");
        let system_info = check_system()?;
        setup(&system_info, config).await?;
        end_group!();
    }
    //TODO: add valgrind version check
    start_opened_group!("Run the benchmarks");
    let profile_folder = create_profile_folder()?;
    let mongo_tracer = if let Some(mongodb_config) = &config.instruments.mongodb {
        let mut mongo_tracer = MongoTracer::try_from(&profile_folder, mongodb_config)?;
        mongo_tracer.start().await?;
        Some(mongo_tracer)
    } else {
        None
    };
    valgrind::measure(config, &profile_folder, &mongo_tracer)?;
    harvest_perf_maps(&profile_folder)?;
    if let Some(mut mongo_tracer) = mongo_tracer {
        mongo_tracer.stop().await?;
    }
    end_group!();
    Ok(RunData { profile_folder })
}
