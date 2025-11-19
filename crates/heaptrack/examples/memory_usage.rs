use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use heaptrack::events::{Event, EventType};

#[derive(Debug, Default)]
struct MemoryStats {
    // Precise tracking via uprobes (malloc/free)
    uprobe_allocated: u64,
    uprobe_freed: u64,
    uprobe_current: u64,
    uprobe_peak: u64,

    // Rough tracking via syscalls (mmap/munmap/brk)
    syscall_mapped: u64,
    syscall_unmapped: u64,
    syscall_current: u64,
    syscall_peak: u64,

    // Track individual allocations for precise accounting
    allocations: HashMap<u64, u64>, // addr -> size
}

impl MemoryStats {
    fn process_event(&mut self, event: &Event) {
        match event.event_type {
            // Uprobe-based precise tracking
            EventType::Malloc | EventType::Calloc | EventType::AlignedAlloc => {
                self.uprobe_allocated += event.size;
                self.uprobe_current += event.size;
                self.allocations.insert(event.addr, event.size);

                if self.uprobe_current > self.uprobe_peak {
                    self.uprobe_peak = self.uprobe_current;
                }
            }
            EventType::Free => {
                if let Some(size) = self.allocations.remove(&event.addr) {
                    self.uprobe_freed += size;
                    self.uprobe_current = self.uprobe_current.saturating_sub(size);
                }
            }
            EventType::Realloc => {
                // For realloc, we need to handle it specially
                // First free the old allocation if it exists
                if let Some(old_size) = self.allocations.remove(&event.addr) {
                    self.uprobe_freed += old_size;
                    self.uprobe_current = self.uprobe_current.saturating_sub(old_size);
                }
                // Then account for the new allocation
                self.uprobe_allocated += event.size;
                self.uprobe_current += event.size;
                self.allocations.insert(event.addr, event.size);

                if self.uprobe_current > self.uprobe_peak {
                    self.uprobe_peak = self.uprobe_current;
                }
            }

            // Syscall-based rough tracking
            EventType::Mmap => {
                self.syscall_mapped += event.size;
                self.syscall_current += event.size;

                if self.syscall_current > self.syscall_peak {
                    self.syscall_peak = self.syscall_current;
                }
            }
            EventType::Munmap => {
                self.syscall_unmapped += event.size;
                self.syscall_current = self.syscall_current.saturating_sub(event.size);
            }
            EventType::Brk => {
                // For brk, we just track it was called
                // The actual size change is difficult to determine without previous state
            }

            EventType::Execve => {
                // Just informational, doesn't affect memory accounting
            }
        }
    }

    fn print_summary(&self) {
        println!("\n=== Memory Usage Summary ===\n");

        println!("Precise Tracking (via uprobes - malloc/free):");
        println!(
            "  Total allocated: {} bytes ({:.2} MB)",
            self.uprobe_allocated,
            self.uprobe_allocated as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Total freed:     {} bytes ({:.2} MB)",
            self.uprobe_freed,
            self.uprobe_freed as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Current usage:   {} bytes ({:.2} MB)",
            self.uprobe_current,
            self.uprobe_current as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Peak usage:      {} bytes ({:.2} MB)",
            self.uprobe_peak,
            self.uprobe_peak as f64 / 1024.0 / 1024.0
        );
        println!("  Outstanding allocations: {}", self.allocations.len());

        println!("\nRough Tracking (via syscalls - mmap/munmap/brk):");
        println!(
            "  Total mapped:    {} bytes ({:.2} MB)",
            self.syscall_mapped,
            self.syscall_mapped as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Total unmapped:  {} bytes ({:.2} MB)",
            self.syscall_unmapped,
            self.syscall_unmapped as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Current mapped:  {} bytes ({:.2} MB)",
            self.syscall_current,
            self.syscall_current as f64 / 1024.0 / 1024.0
        );
        println!(
            "  Peak mapped:     {} bytes ({:.2} MB)",
            self.syscall_peak,
            self.syscall_peak as f64 / 1024.0 / 1024.0
        );
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <allocations.jsonl>", args[0]);
        std::process::exit(1);
    }

    let file = File::open(&args[1])?;
    let reader = BufReader::new(file);

    let mut stats = MemoryStats::default();
    let mut event_count = 0;

    for line in reader.lines() {
        let line = line?;
        let event: Event = serde_json::from_str(&line)?;
        stats.process_event(&event);
        event_count += 1;
    }

    println!("Processed {event_count} events");
    stats.print_summary();

    Ok(())
}
