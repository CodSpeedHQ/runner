use crate::executor::ExecutionContext;
use crate::logger::{GROUP_TARGET, OPENED_GROUP_TARGET};
use crate::prelude::*;
use crate::run_environment::RunEnvironmentProvider;
use log::LevelFilter;
use simplelog::{CombinedLogger, WriteLogger};
use std::fs::copy;
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::NamedTempFile;

static LOGGER_INSTANCE: OnceLock<Result<Logger>> = OnceLock::new();

pub struct Logger {
    log_file_path: PathBuf,
}

impl Logger {
    /// Get or initialize the global logger instance
    #[allow(clippy::borrowed_box)]
    pub fn get_or_init(provider: &Box<dyn RunEnvironmentProvider>) -> Result<&'static Self> {
        LOGGER_INSTANCE
            .get_or_init(|| {
                let provider_logger = provider.get_logger();
                let log_file = match NamedTempFile::new() {
                    Ok(f) => f,
                    Err(e) => {
                        return Err(anyhow::Error::from(e).context("Failed to create log file"));
                    }
                };
                let log_file_path = log_file.path().to_path_buf();
                let file_logger_config = simplelog::ConfigBuilder::new()
                    // Groups are not logged to the file
                    .add_filter_ignore_str(GROUP_TARGET)
                    .add_filter_ignore_str(OPENED_GROUP_TARGET)
                    .build();
                let file_logger =
                    WriteLogger::new(LevelFilter::Trace, file_logger_config, log_file);
                if let Err(e) = CombinedLogger::init(vec![provider_logger, file_logger]) {
                    return Err(anyhow::Error::from(e).context("Failed to init logger"));
                }
                Ok(Self { log_file_path })
            })
            .as_ref()
            .map_err(|e| anyhow!("{e}"))
    }

    pub fn persist_log_to_profile_folder(
        &self,
        execution_context: &ExecutionContext,
    ) -> Result<()> {
        let profile_folder = execution_context.profile_folder.clone();
        let dest_log_file_path = profile_folder.join("runner.log");
        debug!("Persisting log file to {}", dest_log_file_path.display());
        log::logger().flush();
        copy(&self.log_file_path, dest_log_file_path).context("Failed to copy log file")?;
        Ok(())
    }
}
