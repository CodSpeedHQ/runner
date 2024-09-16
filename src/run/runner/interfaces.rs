use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct RunData {
    pub profile_folder: PathBuf,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorName {
    Valgrind,
}
