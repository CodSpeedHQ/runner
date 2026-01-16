use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub mod analysis;
mod prelude;
pub mod uri;
pub mod walltime;

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementMode {
    Walltime,
    Memory,
    Simulation,
}
