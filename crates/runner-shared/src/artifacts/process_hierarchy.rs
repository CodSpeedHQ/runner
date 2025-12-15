use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about a single process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetadata {
    pub pid: i32,
    pub name: String,
    pub start_time: u64,
    pub exit_code: Option<i32>,
    pub stop_time: Option<u64>,
}

/// Tree-like structure tracking process execution hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHierarchy {
    pub root_pid: i32,
    /// Map of PID to process metadata
    pub processes: HashMap<i32, ProcessMetadata>,
    /// Map of parent PID to list of child PIDs
    pub children: HashMap<i32, Vec<i32>>,
}

impl super::ArtifactExt for ProcessHierarchy {}
