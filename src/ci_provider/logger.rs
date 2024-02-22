use lazy_static::lazy_static;

use crate::runner::VALGRIND_EXECUTION_TARGET;

/// This target is used exclusively to handle group events.
pub const GROUP_TARGET: &str = "codspeed::group";
pub const OPENED_GROUP_TARGET: &str = "codspeed::group::opened";

#[macro_export]
/// Start a new log group. All logs between this and the next `end_group!` will be grouped together.
///
/// # Example
///
/// ```rust
/// start_group!("My group");
/// info!("This will be grouped");
/// end_group!();
/// ```
macro_rules! start_group {
    ($name:expr) => {
        log::log!(target: $crate::ci_provider::logger::GROUP_TARGET, log::Level::Info, "{}", $name);
    };
}

#[macro_export]
/// Start a new opened log group. All logs between this and the next `end_group!` will be grouped together.
///
/// # Example
///
/// ```rust
/// start_opened_group!("My group");
/// info!("This will be grouped");
/// end_group!();
/// ```
macro_rules! start_opened_group {
    ($name:expr) => {
        log::log!(target: $crate::ci_provider::logger::OPENED_GROUP_TARGET, log::Level::Info, "{}", $name);
    };
}

#[macro_export]
/// End the current log group.
/// See [`start_group!`] for more information.
macro_rules! end_group {
    () => {
        log::log!(target: $crate::ci_provider::logger::GROUP_TARGET, log::Level::Info, "");
    };
}

pub enum GroupEvent {
    Start(String),
    StartOpened(String),
    End,
}

/// Returns the group event if the record is a group event, otherwise returns `None`.
pub(super) fn get_group_event(record: &log::Record) -> Option<GroupEvent> {
    match record.target() {
        OPENED_GROUP_TARGET => {
            let args = record.args().to_string();
            if args.is_empty() {
                None
            } else {
                Some(GroupEvent::StartOpened(args))
            }
        }
        GROUP_TARGET => {
            let args = record.args().to_string();
            if args.is_empty() {
                Some(GroupEvent::End)
            } else {
                Some(GroupEvent::Start(args))
            }
        }
        _ => None,
    }
}

lazy_static! {
    pub static ref PROVIDER_LOGGER_CONFIG: simplelog::Config = simplelog::ConfigBuilder::new()
    // We don't want to display the execution twice, so we ignore the logs from the provider target
        .add_filter_ignore_str(VALGRIND_EXECUTION_TARGET)
        .build();
}
