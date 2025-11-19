use serde::{Deserialize, Serialize};

// Include the event constants generated from event_constants.h
mod bindings {
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Event {
    pub timestamp: u64,
    pub pid: u32,
    pub event_type: EventType,
    pub addr: u64,
    pub size: u64,
}
