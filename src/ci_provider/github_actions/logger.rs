use crate::ci_provider::logger::{get_group_event, GroupEvent};
use log::*;
use simplelog::SharedLogger;
use std::io::Write;

/// A logger that prints logs in the format expected by GitHub Actions, with grouping support.
///
/// See https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions
pub struct GithubActionLogger;

impl Log for GithubActionLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let level = record.level();
        let message = record.args();

        if let Some(group_event) = get_group_event(record) {
            match group_event {
                GroupEvent::Start(name) | GroupEvent::StartOpened(name) => {
                    println!("::group::{}", name);
                }
                GroupEvent::End => {
                    println!("::endgroup::");
                }
            }
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

impl SharedLogger for GithubActionLogger {
    fn level(&self) -> LevelFilter {
        // since TRACE and DEBUG use ::debug::, we always enable them and let GitHub handle the filtering
        // thanks to https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/enabling-debug-logging#enabling-step-debug-logging
        LevelFilter::Trace
    }

    fn config(&self) -> Option<&simplelog::Config> {
        None
    }

    fn as_log(self: Box<Self>) -> Box<dyn Log> {
        Box::new(*self)
    }
}
