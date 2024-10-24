use std::{env, io::Write};

use log::{LevelFilter, Log};
use simplelog::SharedLogger;

/// A logger that prints log in the format expected by GitLab CI
///
/// See https://docs.gitlab.com/ee/ci/yaml/script.html
pub struct GitLabCILogger {
    log_level: LevelFilter,
}

impl GitLabCILogger {
    pub fn new() -> Self {
        let log_level = env::var("CODSPEED_LOG")
            .ok()
            .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
            .unwrap_or(log::LevelFilter::Info);
        Self { log_level }
    }
}

impl Log for GitLabCILogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, _record: &log::Record) {
        unimplemented!()
    }

    fn flush(&self) {
        std::io::stdout().flush().unwrap();
    }
}

impl SharedLogger for GitLabCILogger {
    fn level(&self) -> LevelFilter {
        self.log_level
    }

    fn config(&self) -> Option<&simplelog::Config> {
        None
    }

    fn as_log(self: Box<Self>) -> Box<dyn Log> {
        Box::new(*self)
    }
}
