use serde::{Deserialize, Serialize};

use crate::fifo::MarkerType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTimestamps {
    pub uri_by_ts: Vec<(u64, String)>,
    pub markers: Vec<MarkerType>,
}
impl super::ArtifactExt for ExecutionTimestamps {}

impl ExecutionTimestamps {
    pub fn new(uri_by_ts: &[(u64, String)], markers: &[crate::fifo::MarkerType]) -> Self {
        Self {
            uri_by_ts: uri_by_ts.to_vec(),
            markers: markers.to_vec(),
        }
    }
}
