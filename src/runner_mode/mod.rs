use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;

mod shell_session;

pub(crate) use shell_session::load_shell_session_mode;
pub(crate) use shell_session::register_shell_session_mode;

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunnerMode {
    #[deprecated(note = "Use `RunnerMode::Simulation` instead")]
    Instrumentation,
    Simulation,
    Walltime,
    Memory,
}
