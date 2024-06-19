use crate::run::runner::VALGRIND_EXECUTION_TARGET;

pub(super) fn should_provider_logger_handle_record(record: &log::Record) -> bool {
    // Provider logger should handle all records except the ones from the valgrind execution target
    record.target() != VALGRIND_EXECUTION_TARGET
}
