//! heaptrack - BPF-based memory allocation tracker
//!
//! This library provides eBPF-based tracking of memory allocations (malloc/free calls)
//! in Linux processes using kernel uprobes.
//!
//! # Quick Start
//!
//! The easiest way to use this library is with the high-level [`track`] function:
//!
//! ```ignore
//! use heaptrack;
//! use std::process::Command;
//!
//! // Spawn a process
//! let child = Command::new("./my_program").spawn()?;
//! let pid = child.id() as i32;
//!
//! // Track its allocations
//! let events = heaptrack::track(pid)?;
//!
//! // Process events
//! for event in events {
//!     println!("Allocation: {:?}", event);
//! }
//! ```
//!
//! # Advanced Usage
//!
//! For more control, use the [`Tracker`] API:
//!
//! ```ignore
//! use heaptrack::Tracker;
//!
//! let tracker = Tracker::new()?;
//! let events = tracker.track_multiple(&[pid1, pid2, pid3])?;
//!
//! for event in events {
//!     println!("Event from PID {}: {:?}", event.pid, event);
//! }
//! ```
//!
//! # Low-level API
//!
//! For full control over the BPF lifecycle, use [`HeaptrackBpf`] directly:
//!
//! ```ignore
//! use heaptrack::HeaptrackBpf;
//!
//! let mut heaptrack = HeaptrackBpf::new()?;
//! heaptrack.attach_malloc("/lib64/libc.so.6".as_ref())?;
//! heaptrack.attach_free("/lib64/libc.so.6".as_ref())?;
//! let (_poller, events) = heaptrack.start_polling_with_channel(10)?;
//! ```

pub mod bpf;
pub mod events;
pub mod libc;
pub mod poller;
pub mod tracker;

pub use bpf::HeaptrackBpf;
pub use events::{Event, EventType};
pub use poller::{EventHandler, RingBufferPoller};
pub use tracker::Tracker;
