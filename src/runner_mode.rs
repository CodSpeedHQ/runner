use clap::ValueEnum;
use serde::Serialize;

#[derive(ValueEnum, Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunnerMode {
    #[deprecated(note = "Use `RunnerMode::Simulation` instead")]
    Instrumentation,
    Simulation,
    Walltime,
    Memory,
}
