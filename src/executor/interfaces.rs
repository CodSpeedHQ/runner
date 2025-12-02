use crate::{
    executor,
    run::{check_system::SystemInfo, logger::Logger},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::run_environment::RunEnvironmentProvider;

pub struct RunData {
    pub profile_folder: PathBuf,
}

pub struct ExecutionContext {
    pub executor_config: executor::Config,
    pub provider: Box<dyn RunEnvironmentProvider>,
    pub logger: Logger,
    pub system_info: SystemInfo,
    pub run_data: RunData,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorName {
    Valgrind,
    WallTime,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ExecutorName {
    fn to_string(&self) -> String {
        match self {
            ExecutorName::Valgrind => "valgrind".to_string(),
            ExecutorName::WallTime => "walltime".to_string(),
        }
    }
}
