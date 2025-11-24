use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct RunData {
    pub profile_folder: PathBuf,
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
