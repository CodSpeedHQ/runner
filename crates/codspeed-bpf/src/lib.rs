//! Shared BPF utilities for CodSpeed tracing tools
//!
//! This crate provides common functionality for BPF-based tracing:
//! - Process hierarchy tracking
//! - Ring buffer polling
//! - Probe attachment macros
//! - Build utilities for BPF programs

pub mod macros;
pub mod poller;
pub mod process_tracking;

#[cfg(feature = "build")]
pub mod build;

pub use poller::{EventHandler, RingBufferPoller};
pub use process_tracking::{ProcessTracking, bump_memlock_rlimit};
