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
    Fork = EVENT_TYPE_FORK as u8,
    Exec = EVENT_TYPE_EXEC as u8,
    Exit = EVENT_TYPE_EXIT as u8,
}

impl From<u8> for EventType {
    fn from(val: u8) -> Self {
        match val as u32 {
            bindings::EVENT_TYPE_FORK => EventType::Fork,
            bindings::EVENT_TYPE_EXEC => EventType::Exec,
            bindings::EVENT_TYPE_EXIT => EventType::Exit,
            _ => panic!("Unknown event type: {val}"),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub event_type: u8,
    pub timestamp: u64,
    pub pid: u32,
    pub tid: u32,
    pub ppid: u32,
    pub comm: [u8; 16],
}

impl Event {
    /// Get the event type as an enum
    pub fn event_type(&self) -> EventType {
        EventType::from(self.event_type)
    }

    /// Get the command name as a string
    pub fn comm_str(&self) -> &str {
        let len = self.comm.iter().position(|&c| c == 0).unwrap_or(16);
        std::str::from_utf8(&self.comm[..len]).unwrap_or("<invalid>")
    }
}

// Static assertions for C/Rust ABI safety
mod assertions {
    use super::*;
    use static_assertions::{assert_eq_align, assert_eq_size, const_assert_eq};
    use std::mem::offset_of;

    assert_eq_size!(Event, bindings::event);
    assert_eq_align!(Event, bindings::event);

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
    const_assert_eq!(offset_of!(Event, comm), offset_of!(bindings::event, comm));
}
