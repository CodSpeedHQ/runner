use libc::pid_t;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemtrackArtifact {
    pub events: Vec<MemtrackEvent>,
}
impl super::ArtifactExt for MemtrackArtifact {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemtrackEvent {
    pub pid: pid_t,
    pub tid: pid_t,
    pub timestamp: u64,
    pub addr: u64,
    #[serde(flatten)]
    pub kind: MemtrackEventKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MemtrackEventKind {
    Malloc { size: u64 },
    Free,
    Realloc { size: u64 },
    Calloc { size: u64 },
    AlignedAlloc { size: u64 },
    Mmap { size: u64 },
    Munmap { size: u64 },
    Brk { size: u64 },
}
