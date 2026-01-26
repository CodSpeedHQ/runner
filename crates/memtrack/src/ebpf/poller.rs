use anyhow::Result;
use libbpf_rs::{MapCore, RingBufferBuilder};
use runner_shared::artifacts::MemtrackEvent as Event;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use super::events::parse_event;

/// A handler function for processing ring buffer events
pub type EventHandler = Box<dyn Fn(Event) + Send>;

/// RingBufferPoller manages polling a BPF ring buffer in a background thread
/// and sending events to handlers
pub struct RingBufferPoller {
    shutdown: Arc<AtomicBool>,
    poll_thread: Option<JoinHandle<()>>,
}

impl RingBufferPoller {
    /// Create a new RingBufferPoller for the given ring buffer map
    ///
    /// # Arguments
    /// * `rb_map` - The BPF ring buffer map to poll
    /// * `handler` - Callback function to handle each event
    /// * `poll_timeout_ms` - How long to wait for events in each poll iteration
    pub fn new<M: MapCore + 'static>(
        rb_map: &M,
        handler: EventHandler,
        poll_timeout_ms: u64,
    ) -> Result<Self> {
        let mut builder = RingBufferBuilder::new();
        builder.add(rb_map, move |data| {
            if let Some(event) = parse_event(data) {
                handler(event);
            }
            0
        })?;

        let ringbuf = builder.build()?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let poll_thread = std::thread::spawn(move || {
            while !shutdown_clone.load(Ordering::Relaxed) {
                let _ = ringbuf.poll(Duration::from_millis(poll_timeout_ms));
            }
        });

        Ok(Self {
            shutdown,
            poll_thread: Some(poll_thread),
        })
    }

    /// Create a new RingBufferPoller with an mpsc channel for events
    ///
    /// Returns the RingBufferPoller and the receiver end of the channel
    pub fn with_channel<M: MapCore + 'static>(
        rb_map: &M,
        poll_timeout_ms: u64,
    ) -> Result<(Self, mpsc::Receiver<Event>)> {
        let (tx, rx) = mpsc::channel();
        let poller = Self::new(
            rb_map,
            Box::new(move |event| {
                let _ = tx.send(event);
            }),
            poll_timeout_ms,
        )?;
        Ok((poller, rx))
    }

    /// Stop the polling thread and wait for it to finish
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(thread) = self.poll_thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for RingBufferPoller {
    fn drop(&mut self) {
        self.shutdown();
    }
}
