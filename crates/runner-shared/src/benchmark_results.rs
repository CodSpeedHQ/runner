use crate::fifo::MarkerType;
use libc::pid_t;
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTimestamps {
    pub uri_by_ts: Vec<(u64, String)>,
    pub markers: Vec<MarkerType>,
}
impl ArtifactExt for ExecutionTimestamps {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemtrackArtifact {
    pub events: Vec<MemtrackEvent>,
}
impl ArtifactExt for MemtrackArtifact {}

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

pub trait ArtifactExt
where
    Self: Sized + Serialize,
{
    /// WARNING: This doesn't support generic types
    fn name() -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }

    fn save_file_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        filename: &str,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(folder.as_ref())?;
        let data = rmp_serde::to_vec_named(self)?;
        std::fs::write(folder.as_ref().join(filename), data)?;

        debug!("Saved {} result to {:?}", Self::name(), folder.as_ref());
        Ok(())
    }

    fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{}.msgpack", Self::name()))
    }

    fn save_with_pid_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        pid: pid_t,
    ) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{pid}.{}.msgpack", Self::name()))
    }
}

impl ExecutionTimestamps {
    pub fn new(uri_by_ts: &[(u64, String)], markers: &[crate::fifo::MarkerType]) -> Self {
        Self {
            uri_by_ts: uri_by_ts.to_vec(),
            markers: markers.to_vec(),
        }
    }
}
