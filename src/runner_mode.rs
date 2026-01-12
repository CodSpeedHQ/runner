use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunnerMode {
    #[deprecated(note = "Use `RunnerMode::Simulation` instead")]
    Instrumentation,
    Simulation,
    Walltime,
    Memory,
}
