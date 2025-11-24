use crate::run::executor::EXECUTOR_TARGET;

pub(super) fn should_provider_logger_handle_record(record: &log::Record) -> bool {
    // Provider logger should handle all records except the ones from the executor target
    record.target() != EXECUTOR_TARGET
}
