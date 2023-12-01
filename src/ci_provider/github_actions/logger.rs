use log::*;

use crate::ci_provider::logger::{get_group_event, GroupEvent};

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

    fn flush(&self) {}
}
