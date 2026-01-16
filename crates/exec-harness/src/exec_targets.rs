use serde::{Deserialize, Serialize};

/// File format for passing multiple exec targets from runner to exec-harness
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecTargetsFile {
    pub targets: Vec<ExecTarget>,
}

/// A single execution target with merged options
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecTarget {
    /// Optional benchmark name (derived from command if not set)
    pub name: Option<String>,
    /// Command and arguments to execute
    pub command: Vec<String>,
    /// Merged walltime execution options
    pub walltime_options: WalltimeExecutionOptions,
}

/// Walltime execution options in a serializable format
///
/// All durations are stored as nanoseconds (already parsed).
/// This avoids the need for exec-harness to re-parse duration strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalltimeExecutionOptions {
    /// Warmup time in nanoseconds
    pub warmup_time_ns: Option<u64>,
    /// Maximum execution time in nanoseconds
    pub max_time_ns: Option<u64>,
    /// Minimum execution time in nanoseconds
    pub min_time_ns: Option<u64>,
    /// Maximum number of rounds
    pub max_rounds: Option<u64>,
    /// Minimum number of rounds
    pub min_rounds: Option<u64>,
}

impl Default for WalltimeExecutionOptions {
    fn default() -> Self {
        Self {
            warmup_time_ns: None,
            max_time_ns: None,
            min_time_ns: None,
            max_rounds: None,
            min_rounds: None,
        }
    }
}
