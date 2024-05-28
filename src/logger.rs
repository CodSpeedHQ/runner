use std::env;

use simplelog::{ConfigBuilder, SharedLogger};

pub fn get_local_logger() -> Box<dyn SharedLogger> {
    let log_level = env::var("CODSPEED_LOG")
        .ok()
        .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Info);

    let config = ConfigBuilder::new()
        .set_time_level(log::LevelFilter::Debug)
        .build();

    simplelog::TermLogger::new(
        log_level,
        config,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
}
