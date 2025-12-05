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
    Execve = EVENT_TYPE_EXECVE as u8,
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
            bindings::EVENT_TYPE_EXECVE => EventType::Execve,
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

// TODO: Can't we use the bindgen generated type?
#[repr(C)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub pid: i32,
    pub tid: i32,
    pub addr: u64,
    pub size: u64,
}

impl From<Event> for MemtrackEvent {
    fn from(val: Event) -> Self {
        let kind = match val.event_type {
            EventType::Malloc => MemtrackEventKind::Malloc { size: val.size },
            EventType::Free => MemtrackEventKind::Free,
            EventType::Calloc => MemtrackEventKind::Calloc { size: val.size },
            EventType::Realloc => MemtrackEventKind::Realloc { size: val.size },
            EventType::AlignedAlloc => MemtrackEventKind::AlignedAlloc { size: val.size },
            EventType::Mmap => MemtrackEventKind::Mmap { size: val.size },
            EventType::Munmap => MemtrackEventKind::Munmap { size: val.size },
            EventType::Brk => MemtrackEventKind::Brk { size: val.size },
            _ => panic!("This event isn't meant to be used outside of memtrack"),
        };

        MemtrackEvent {
            pid: val.pid,
            tid: val.tid,
            timestamp: val.timestamp,
            addr: val.addr,
            kind,
        }
    }
}

mod assertions {
    use super::*;
    use static_assertions::{assert_eq_align, assert_eq_size, const_assert_eq};
    use std::mem::offset_of;

    // Verify size and alignment match the bindgen-generated event struct
    assert_eq_size!(Event, bindings::event);
    assert_eq_align!(Event, bindings::event);

    // Verify field offsets match the C struct
    const_assert_eq!(
        offset_of!(Event, timestamp),
        offset_of!(bindings::event, timestamp)
    );
    const_assert_eq!(offset_of!(Event, pid), offset_of!(bindings::event, pid));
    const_assert_eq!(offset_of!(Event, tid), offset_of!(bindings::event, tid));
    const_assert_eq!(
        offset_of!(Event, event_type),
        offset_of!(bindings::event, event_type)
    );
    const_assert_eq!(offset_of!(Event, addr), offset_of!(bindings::event, addr));
    const_assert_eq!(offset_of!(Event, size), offset_of!(bindings::event, size));
}
