use crate::logger::{GROUP_TARGET, OPENED_GROUP_TARGET};
use crate::prelude::*;
use crate::run::{ci_provider::CIProvider, runner::RunData};
use log::LevelFilter;
use simplelog::{CombinedLogger, WriteLogger};
use std::fs::copy;
use std::path::PathBuf;
use tempfile::NamedTempFile;

pub struct Logger {
    log_file_path: PathBuf,
}

impl Logger {
    #[allow(clippy::borrowed_box)]
    pub fn new(provider: &Box<dyn CIProvider>) -> Result<Self> {
        let provider_logger = provider.get_logger();
        let log_file = NamedTempFile::new().context("Failed to create log file")?;
        let log_file_path = log_file.path().to_path_buf();
        let file_logger_config = simplelog::ConfigBuilder::new()
            // Groups are not logged to the file
            .add_filter_ignore_str(GROUP_TARGET)
            .add_filter_ignore_str(OPENED_GROUP_TARGET)
            .build();
        let file_logger = WriteLogger::new(LevelFilter::Trace, file_logger_config, log_file);
        CombinedLogger::init(vec![provider_logger, file_logger])
            .context("Failed to init logger")?;
        Ok(Self { log_file_path })
    }

    pub fn persist_log_to_profile_folder(&self, run_data: &RunData) -> Result<()> {
        let profile_folder = run_data.profile_folder.clone();
        let dest_log_file_path = profile_folder.join("runner.log");
        debug!("Persisting log file to {}", dest_log_file_path.display());
        log::logger().flush();
        copy(&self.log_file_path, dest_log_file_path).context("Failed to copy log file")?;
        Ok(())
    }
}
