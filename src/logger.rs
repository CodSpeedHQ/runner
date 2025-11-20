/// This target is used exclusively to handle group events.
pub const GROUP_TARGET: &str = "codspeed::group";
pub const OPENED_GROUP_TARGET: &str = "codspeed::group::opened";
pub const ANNOUNCEMENT_TARGET: &str = "codspeed::announcement";

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
        log::log!(target: $crate::logger::GROUP_TARGET, log::Level::Info, "{}", $name);
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
        log::log!(target: $crate::logger::OPENED_GROUP_TARGET, log::Level::Info, "{}", $name);
    };
}

#[macro_export]
/// End the current log group.
/// See [`start_group!`] for more information.
macro_rules! end_group {
    () => {
        log::log!(target: $crate::logger::GROUP_TARGET, log::Level::Info, "");
    };
}

#[macro_export]
/// Logs at the announcement level. This is intended for important announcements like new features,
/// that do not require immediate user action.
macro_rules! announcement {
    ($name:expr) => {
        log::log!(target: $crate::logger::ANNOUNCEMENT_TARGET, log::Level::Info, "{}", $name);
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

pub(super) fn get_announcement_event(record: &log::Record) -> Option<String> {
    if record.target() != ANNOUNCEMENT_TARGET {
        return None;
    }

    Some(record.args().to_string())
}

#[macro_export]
/// Log a structured JSON output
macro_rules! log_json {
    ($value:expr) => {
        log::log!(target: $crate::logger::JSON_TARGET, log::Level::Info, "{}", $value);
    };
}

pub struct JsonEvent(pub String);

pub const JSON_TARGET: &str = "codspeed::json";

pub(super) fn get_json_event(record: &log::Record) -> Option<JsonEvent> {
    if record.target() != JSON_TARGET {
        return None;
    }

    Some(JsonEvent(record.args().to_string()))
}
