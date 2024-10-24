use log::{Level, LevelFilter, Log};
use simplelog::SharedLogger;
use std::{
    env,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    logger::{get_group_event, GroupEvent},
    run::ci_provider::logger::should_provider_logger_handle_record,
};

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

    fn log(&self, record: &log::Record) {
        if !should_provider_logger_handle_record(record) {
            return;
        }

        let level = record.level();
        let message = record.args();

        if let Some(group_event) = get_group_event(record) {
            let now = SystemTime::now();
            let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();

            match group_event {
                GroupEvent::Start(name) | GroupEvent::StartOpened(name) => {
                    println!("section_start:{timestamp}:{name}");
                }
                GroupEvent::End => {
                    println!("section_end:{timestamp}");
                }
            }
            return;
        }

        if level > self.log_level {
            return;
        }

        let prefix = match level {
            Level::Error => "::error::",
            Level::Warn => "::warning::",
            Level::Info => "",
            Level::Debug => "::debug::",
            Level::Trace => "::debug::[TRACE]",
        };
        let message_string = message.to_string();
        let lines = message_string.lines();
        // ensure that all the lines of the message have the prefix, otherwise GitHub Actions will not recognize the command for the whole string
        lines.for_each(|line| println!("{}{}", prefix, line));
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
