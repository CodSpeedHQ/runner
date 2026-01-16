use crate::runner_mode::RunnerMode;
use serde::{Deserialize, Serialize};

/// Project-level configuration from codspeed.yaml file
///
/// This configuration provides default options for the run and exec commands.
/// CLI arguments always take precedence over config file values.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectConfig {
    /// Default options to apply to all benchmark runs
    pub options: Option<ProjectOptions>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct TargetOptions {
    pub walltime: Option<WalltimeOptions>,
}

/// Root-level options that apply to all benchmark runs unless overridden by CLI
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectOptions {
    /// Walltime execution configuration
    pub walltime: Option<WalltimeOptions>,
    /// Working directory where commands will be executed (relative to config file)
    pub working_directory: Option<String>,
    /// Runner mode (walltime, memory, or simulation)
    pub mode: Option<RunnerMode>,
}

/// Walltime execution options matching WalltimeExecutionArgs structure
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct WalltimeOptions {
    /// Duration of warmup phase (e.g., "1s", "500ms")
    pub warmup_time: Option<String>,
    /// Maximum total execution time
    pub max_time: Option<String>,
    /// Minimum total execution time
    pub min_time: Option<String>,
    /// Maximum number of rounds
    pub max_rounds: Option<u64>,
    /// Minimum number of rounds
    pub min_rounds: Option<u64>,
}
