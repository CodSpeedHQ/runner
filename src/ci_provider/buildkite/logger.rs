use log::*;

use crate::ci_provider::logger::{get_group_event, GroupEvent};

/// A logger that prints logs in the format expected by Buildkite
///
/// See https://buildkite.com/docs/pipelines/managing-log-output
pub struct BuildkiteLogger;

impl Log for BuildkiteLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let level = record.level();
        let message = record.args();

        if let Some(group_event) = get_group_event(record) {
            match group_event {
                GroupEvent::Start(name) => {
                    println!("--- {}", name);
                }
                GroupEvent::StartOpened(name) => {
                    println!("+++ {}", name);
                }
                GroupEvent::End => {}
            }
            return;
        }

        // there is no support for log levels in Buildkite, so we print the level in the message
        match level {
            Level::Error => {
                println!("[ERROR] {}", message);
            }
            Level::Warn => {
                println!("[WARN] {}", message);
            }
            Level::Info => {
                println!("{}", message);
            }
            Level::Debug => {
                println!("[DEBUG] {}", message);
            }
            Level::Trace => {
                println!("[TRACE] {}", message);
            }
        }
    }

    fn flush(&self) {}
}
