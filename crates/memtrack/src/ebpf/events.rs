use runner_shared::artifacts::{MemtrackEvent, MemtrackEventKind};
use serde::{Deserialize, Serialize};

// Include the bindings for event.h
pub mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/event.rs"));
}
use bindings::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum EventType {
    Malloc = EVENT_TYPE_MALLOC as u8,
    Free = EVENT_TYPE_FREE as u8,
    Calloc = EVENT_TYPE_CALLOC as u8,
    Realloc = EVENT_TYPE_REALLOC as u8,
    AlignedAlloc = EVENT_TYPE_ALIGNED_ALLOC as u8,
    Mmap = EVENT_TYPE_MMAP as u8,
    Munmap = EVENT_TYPE_MUNMAP as u8,
    Brk = EVENT_TYPE_BRK as u8,
}

impl From<u8> for EventType {
    fn from(val: u8) -> Self {
        match val as u32 {
            bindings::EVENT_TYPE_MALLOC => EventType::Malloc,
            bindings::EVENT_TYPE_FREE => EventType::Free,
            bindings::EVENT_TYPE_CALLOC => EventType::Calloc,
            bindings::EVENT_TYPE_REALLOC => EventType::Realloc,
            bindings::EVENT_TYPE_ALIGNED_ALLOC => EventType::AlignedAlloc,
            bindings::EVENT_TYPE_MMAP => EventType::Mmap,
            bindings::EVENT_TYPE_MUNMAP => EventType::Munmap,
            bindings::EVENT_TYPE_BRK => EventType::Brk,
            _ => panic!("Unknown event type: {val}"),
        }
    }
}

/// Extension trait for MemtrackEvent to get the EventType
pub trait MemtrackEventExt {
    fn event_type(&self) -> EventType;
}

impl MemtrackEventExt for MemtrackEvent {
    fn event_type(&self) -> EventType {
        match self.kind {
            MemtrackEventKind::Malloc { .. } => EventType::Malloc,
            MemtrackEventKind::Free => EventType::Free,
            MemtrackEventKind::Calloc { .. } => EventType::Calloc,
            MemtrackEventKind::Realloc { .. } => EventType::Realloc,
            MemtrackEventKind::AlignedAlloc { .. } => EventType::AlignedAlloc,
            MemtrackEventKind::Mmap { .. } => EventType::Mmap,
            MemtrackEventKind::Munmap { .. } => EventType::Munmap,
            MemtrackEventKind::Brk { .. } => EventType::Brk,
        }
    }
}

/// Parse an event from raw bytes into MemtrackEvent
///
/// SAFETY: The data must be a valid `bindings::event`
pub fn parse_event(data: &[u8]) -> Option<MemtrackEvent> {
    if data.len() < std::mem::size_of::<bindings::event>() {
        return None;
    }

    let event = unsafe { &*(data.as_ptr() as *const bindings::event) };
    let event_type = EventType::from(event.header.event_type);

    // Common fields from header
    let pid = event.header.pid as i32;
    let tid = event.header.tid as i32;
    let timestamp = event.header.timestamp;

    // Parse event data based on type
    // SAFETY: The fields must be properly initialized in eBPF
    let (addr, kind) = unsafe {
        match event_type {
            EventType::Malloc => (
                event.data.alloc.addr,
                MemtrackEventKind::Malloc {
                    size: event.data.alloc.size,
                },
            ),
            EventType::Free => (event.data.free.addr, MemtrackEventKind::Free),
            EventType::Calloc => (
                event.data.alloc.addr,
                MemtrackEventKind::Calloc {
                    size: event.data.alloc.size,
                },
            ),
            EventType::Realloc => (
                event.data.realloc.new_addr,
                MemtrackEventKind::Realloc {
                    old_addr: Some(event.data.realloc.old_addr),
                    size: event.data.realloc.size,
                },
            ),
            EventType::AlignedAlloc => (
                event.data.alloc.addr,
                MemtrackEventKind::AlignedAlloc {
                    size: event.data.alloc.size,
                },
            ),
            EventType::Mmap => (
                event.data.mmap.addr,
                MemtrackEventKind::Mmap {
                    size: event.data.mmap.size,
                },
            ),
            EventType::Munmap => (
                event.data.mmap.addr,
                MemtrackEventKind::Munmap {
                    size: event.data.mmap.size,
                },
            ),
            EventType::Brk => (
                event.data.mmap.addr,
                MemtrackEventKind::Brk {
                    size: event.data.mmap.size,
                },
            ),
        }
    };

    Some(MemtrackEvent {
        pid,
        tid,
        timestamp,
        addr,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_realloc_event() {
        // Create a mock event with realloc data
        let mut event: bindings::event = unsafe { std::mem::zeroed() };
        event.header.event_type = bindings::EVENT_TYPE_REALLOC as u8;
        event.header.timestamp = 12345678;
        event.header.pid = 1000;
        event.header.tid = 2000;
        event.data.realloc.old_addr = 0x1000;
        event.data.realloc.new_addr = 0x2000;
        event.data.realloc.size = 256;

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &event as *const _ as *const u8,
                std::mem::size_of_val(&event),
            )
        };

        // Parse and validate:
        let parsed = parse_event(bytes).unwrap();
        assert_eq!(parsed.pid, 1000);
        assert_eq!(parsed.tid, 2000);
        assert_eq!(parsed.timestamp, 12345678);
        assert_eq!(parsed.addr, 0x2000);

        match parsed.kind {
            MemtrackEventKind::Realloc { old_addr, size } => {
                assert_eq!(old_addr, Some(0x1000));
                assert_eq!(size, 256);
            }
            _ => panic!("Expected Realloc event kind"),
        }
    }

    #[test]
    fn test_parse_malloc_event() {
        // Create a mock event with malloc data
        let mut event: bindings::event = unsafe { std::mem::zeroed() };
        event.header.event_type = bindings::EVENT_TYPE_MALLOC as u8;
        event.header.timestamp = 12345678;
        event.header.pid = 1000;
        event.header.tid = 2000;
        event.data.alloc.addr = 0x1000;
        event.data.alloc.size = 128;

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &event as *const _ as *const u8,
                std::mem::size_of_val(&event),
            )
        };

        // Parse and validate:
        let parsed = parse_event(bytes).unwrap();
        assert_eq!(parsed.pid, 1000);
        assert_eq!(parsed.tid, 2000);
        assert_eq!(parsed.timestamp, 12345678);
        assert_eq!(parsed.addr, 0x1000);

        match parsed.kind {
            MemtrackEventKind::Malloc { size } => {
                assert_eq!(size, 128);
            }
            _ => panic!("Expected Malloc event kind"),
        }
    }
}
