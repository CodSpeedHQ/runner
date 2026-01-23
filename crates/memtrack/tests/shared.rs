#![allow(dead_code, unused)]

use anyhow::Context;
use memtrack::prelude::*;
use memtrack::{AllocatorLib, EventType, MemtrackEventExt, Tracker};
use runner_shared::artifacts::{MemtrackEvent as Event, MemtrackEventKind};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

type TrackResult = anyhow::Result<(Vec<Event>, std::thread::JoinHandle<()>)>;

macro_rules! assert_events_with_marker {
    ($name:expr, $events:expr) => {{
        use itertools::Itertools;
        use memtrack::MemtrackEventExt;
        use runner_shared::artifacts::MemtrackEventKind;

        // Dedup events by address and type to remove duplicates
        let events = $events
            .iter()
            .sorted_by_key(|e| e.timestamp)
            .dedup_by(|a, b| a.addr == b.addr && a.event_type() == b.event_type());

        // Remove events outside our 0xC0D59EED marker allocations
        let events = events
            .sorted_by_key(|e| e.timestamp)
            .skip_while(|e| {
                let MemtrackEventKind::Malloc { size } = e.kind else {
                    return true;
                };
                size != 0xC0D59EED
            })
            .skip(2) // Skip the marker allocation and free
            .take_while(|e| {
                let MemtrackEventKind::Malloc { size } = e.kind else {
                    return true;
                };
                size != 0xC0D59EED
            })
            .collect::<Vec<_>>();

        let formatted_events: Vec<String> = events
            .iter()
            .map(|e| match e.kind {
                // Exclude address in snapshots:
                MemtrackEventKind::Realloc { size, .. } => format!("Realloc {{ size: {} }}", size),
                _ => format!("{:?}", e.kind),
            })
            .collect();
        insta::assert_debug_snapshot!($name, formatted_events);
    }};
}

pub fn track_binary_with_opts(binary: &Path, extra_allocators: &[AllocatorLib]) -> TrackResult {
    // IMPORTANT: Always initialize the tracker BEFORE spawning the binary, as it can take some time to
    // attach to all the allocator libraries (especially when using NixOS).
    let mut tracker = memtrack::Tracker::new()?;
    tracker.attach_allocators(extra_allocators)?;

    let child = Command::new(binary)
        .spawn()
        .context("Failed to spawn command")?;
    let root_pid = child.id() as i32;

    tracker.enable()?;
    let rx = tracker.track(root_pid)?;

    let mut events = Vec::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(10)) {
        events.push(event);
    }

    // Drop the tracker in a new thread to not block the test
    let thread_handle = std::thread::spawn(move || core::mem::drop(tracker));

    info!("Tracked {} events", events.len());
    trace!("Events: {events:#?}");

    Ok((events, thread_handle))
}

pub fn track_binary(binary: &Path) -> TrackResult {
    track_binary_with_opts(binary, &[])
}

/// Helper to count events of a specific type
pub fn count_events_by_type(events: &[Event], event_type: EventType) -> usize {
    events
        .iter()
        .filter(|e| e.event_type() == event_type)
        .count()
}
