# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build in release mode (required for BPF programs)
cargo build --release

# Run tests (requires sudo due to BPF requirements)
cargo build --release
sudo cargo test -- --ignored --test-threads=1

# Run the memtrack CLI to track memory allocations
cargo build --release
sudo ./target/release/codspeed-memtrack track <command> [args...] --output <file.jsonl>

# Or avoid sudo by granting capabilities (Linux 5.8+)
sudo setcap cap_bpf,cap_perfmon+ep ./target/release/codspeed-memtrack
./target/release/codspeed-memtrack track <command> --output <file.jsonl>

# Run with debug logging
CODSPEED_LOG=debug sudo ./target/release/codspeed-memtrack track <command>
```

### Linting and Formatting
```bash
# Check code formatting
cargo fmt -- --check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

### Running Examples

#### CLI Examples
```bash
# Run the allocs example (demonstrates various allocation patterns)
cargo build --release && sudo ./target/release/codspeed-memtrack track ./target/release/examples/allocs --output allocs_output.jsonl

# Run the spawn example (tests child process tracking)
cargo build --release && sudo ./target/release/codspeed-memtrack track ./target/release/examples/spawn --output spawn_output.jsonl

# Run the visualize example (generates an image from allocation data)
cargo build --release && sudo ./target/release/codspeed-memtrack track ./target/release/examples/visualize --output vis_output.jsonl
```

#### Library API Examples
```bash
# Run the library_usage example (demonstrates tracking a spawned process)
cargo build --release --examples && sudo ./target/release/examples/library_usage

# Run the track_multiple example (demonstrates tracking multiple processes)
cargo build --release --examples && sudo ./target/release/examples/track_multiple
```

## Architecture Overview

This crate implements a BPF-based memory allocation tracker that monitors malloc/free calls and other memory allocation functions in Linux processes using eBPF uprobes.

### Core Components

1. **High-Level API** (`src/tracker.rs`):
   - `Tracker`: High-level API for tracking processes by PID
   - `track(pid)`: Convenience function to track a single process
   - `track_multiple(pids)`: Track multiple processes simultaneously
   - Returns `Receiver<Event>` for consuming allocation events
   - Handles BPF initialization, libc attachment, and event polling automatically

2. **BPF Layer** (`src/bpf.rs` and `src/bpf/*.c`):
   - `MemtrackBpf`: Main interface for managing BPF programs and probes
   - Attaches uprobes to libc functions: `malloc`, `free`, `calloc`, `realloc`, `aligned_alloc`
   - Attaches to `sched_process_fork` tracepoint for process hierarchy tracking
   - Uses BPF maps to track PIDs and ring buffers to send events to userspace
   - Built with libbpf and generated into skeleton code at build time

3. **Event System** (`src/events.rs`):
   - `Event`: Serializable struct containing allocation events
   - `EventType`: Enum distinguishing allocation types (Malloc, Free, Calloc, Realloc, AlignedAlloc, Execve)
   - Generated constants from `src/bpf/event_constants.h` compiled during build

4. **Ring Buffer Poller** (`src/poller.rs`):
   - `RingBufferPoller`: Manages BPF ring buffer polling in a background thread
   - `EventHandler`: Callback-based or channel-based event processing
   - Graceful shutdown with atomic flags and thread joining

5. **LibC Path Detection** (`src/libc.rs`):
   - Finds libc.so.6 across standard Linux paths and NixOS nix/store
   - Handles multiple glibc versions in the same system

6. **Command Interface** (`src/main.rs`):
   - CLI entry point with `track` subcommand
   - Uses the high-level `Tracker` API internally
   - Spawns target process, writes JSONL output
   - Manages process lifetime and output file handling

### BPF Program Details

The BPF kernel code (`src/bpf/memtrack.bpf.c`) implements:
- **PID Tracking**: Tracks directly added PIDs and auto-tracks child processes via fork tracepoint
- **Process Hierarchy**: Maintains parent-child relationships to track process trees
- **Memory Maps**:
  - `tracked_pids`: Hash map of PIDs to track (10000 entries)
  - `pids_ppid`: Parent PID mapping for process hierarchy
  - `events`: Ring buffer (256KB) for sending events to userspace
- **Uprobes**: Attached to libc allocation functions to capture event data

### Build Process

The `build.rs` script:
1. Generates BPF skeleton from `memtrack.bpf.c` using libbpf-cargo
2. Generates Rust bindings from `event_constants.h` using bindgen
3. Outputs generated code to `OUT_DIR`

## Key Concepts

### Ring Buffer Polling
The poller runs in a background thread with a configurable timeout (in main.rs, set to 10ms). Events are pulled from the BPF ring buffer and sent through an mpsc channel to the main thread for writing to the output file.

### Memory Locking
The process bumps the `RLIMIT_MEMLOCK` resource limit to infinity to allow BPF ring buffer allocation (256KB per CPU).

### JSONL Output
Events are serialized to JSONL (JSON Lines) format with one `Event` per line for streaming processing and easy parsing.

## Using memtrack as a Library

memtrack can be used as a library to track memory allocations in your own Rust programs:

```rust
use memtrack;
use std::process::Command;

// Spawn a process
let child = Command::new("./my_program").spawn()?;
let pid = child.id() as i32;

// Track its allocations
let events = memtrack::track(pid)?;

// Process events
for event in events {
    println!("Allocation: {:?}", event);
}
```

For more examples, see `examples/library_usage.rs` and `examples/track_multiple.rs`.

## Testing

- Integration tests in `tests/integration_test.rs` require sudo to run
- Run with: `cargo build --release && sudo cargo test -- --ignored --test-threads=1`
- Examples in `examples/` demonstrate allocation patterns that can be tracked
- Library API examples show how to use memtrack programmatically

## Important Notes

- **Requires elevated privileges**: BPF programs and uprobes require root or CAP_BPF/CAP_PERFMON capabilities
  - On Linux 5.8+, you can avoid sudo by granting capabilities: `sudo setcap cap_bpf,cap_perfmon+ep <binary>`
- **Linux-only**: Uses BPF/uprobes which are Linux-specific
- **CPU count**: Ring buffer sizes scale with CPU count in the BPF kernel code
- **NixOS compatibility**: Includes special handling for finding libc in NixOS nix/store
