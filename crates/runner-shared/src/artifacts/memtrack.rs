use libc::pid_t;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemtrackArtifact {
    pub events: Vec<MemtrackEvent>,
}
impl super::ArtifactExt for MemtrackArtifact {
    fn encode_to_writer<W: Write>(&self, writer: W) -> anyhow::Result<()> {
        // This is required for `decode_streamed`: We can't stream the deserialization of
        // the whole artifact, so we have to encode them one by one.
        let mut serializer = rmp_serde::Serializer::new(writer);
        for event in &self.events {
            event.serialize(&mut serializer)?;
        }
        Ok(())
    }
}

impl MemtrackArtifact {
    pub fn decode_streamed<R: std::io::Read>(reader: R) -> anyhow::Result<MemtrackEventStream<R>> {
        Ok(MemtrackEventStream {
            deserializer: rmp_serde::Deserializer::new(reader),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemtrackEvent {
    pub pid: pid_t,
    pub tid: pid_t,
    pub timestamp: u64,
    pub addr: u64,
    #[serde(flatten)]
    pub kind: MemtrackEventKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

pub struct MemtrackEventStream<R: Read> {
    deserializer: rmp_serde::Deserializer<rmp_serde::decode::ReadReader<R>>,
}

impl<R: Read> Iterator for MemtrackEventStream<R> {
    type Item = MemtrackEvent;

    fn next(&mut self) -> Option<Self::Item> {
        MemtrackEvent::deserialize(&mut self.deserializer).ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::ArtifactExt;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_decode_streamed() -> anyhow::Result<()> {
        let events = vec![
            MemtrackEvent {
                pid: 1,
                tid: 11,
                timestamp: 100,
                addr: 0x10,
                kind: MemtrackEventKind::Malloc { size: 64 },
            },
            MemtrackEvent {
                pid: 1,
                tid: 12,
                timestamp: 200,
                addr: 0x20,
                kind: MemtrackEventKind::Free,
            },
        ];

        let artifact = MemtrackArtifact {
            events: events.clone(),
        };
        let mut buf = Vec::new();
        artifact.encode_to_writer(&mut buf)?;

        let stream = MemtrackArtifact::decode_streamed(Cursor::new(buf))?;
        let collected: Vec<_> = stream.collect();
        assert_eq!(collected, events);

        Ok(())
    }
}
