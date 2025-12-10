use crate::ebpf::{Event, EventType};
use runner_shared::artifacts::{ProcessHierarchy, ProcessMetadata};
use std::collections::HashMap;

/// Builds and maintains a ProcessHierarchy from eBPF events
pub struct HierarchyBuilder {
    hierarchy: ProcessHierarchy,
    /// Maps PID to parent PID
    parent_map: HashMap<i32, i32>,
}

impl HierarchyBuilder {
    /// Create a new hierarchy builder for a root PID
    pub fn new(root_pid: i32) -> Self {
        Self {
            hierarchy: ProcessHierarchy {
                root_pid,
                processes: HashMap::new(),
                children: HashMap::new(),
            },
            parent_map: HashMap::new(),
        }
    }

    /// Process an event and update the hierarchy
    pub fn process_event(&mut self, event: &Event) {
        let pid = event.pid as i32;
        let comm = event.comm_str().to_string();
        let timestamp = event.timestamp;

        match event.event_type() {
            EventType::Exec => {
                // Update or create process metadata on exec
                self.hierarchy
                    .processes
                    .entry(pid)
                    .and_modify(|p| p.name = comm.clone())
                    .or_insert_with(|| ProcessMetadata {
                        pid,
                        name: comm,
                        start_time: timestamp,
                        exit_code: None,
                        stop_time: None,
                    });
            }
            EventType::Fork => {
                // Fork event - establish parent-child relationship
                let ppid = event.ppid as i32;

                // Create or update process metadata for the child
                if !self.hierarchy.processes.contains_key(&pid) {
                    self.hierarchy.processes.insert(
                        pid,
                        ProcessMetadata {
                            pid,
                            name: comm,
                            start_time: timestamp,
                            exit_code: None,
                            stop_time: None,
                        },
                    );
                }

                // Register the parent-child relationship
                self.register_parent_child(ppid, pid);
            }
            EventType::Exit => {
                // Update process metadata with exit info
                if let Some(metadata) = self.hierarchy.processes.get_mut(&pid) {
                    metadata.exit_code = Some(event.tid as i32); // Exit code in tid field
                    metadata.stop_time = Some(timestamp);
                }
            }
        }
    }

    /// Register a parent-child relationship
    pub fn register_parent_child(&mut self, parent_pid: i32, child_pid: i32) {
        self.parent_map.insert(child_pid, parent_pid);
        self.hierarchy
            .children
            .entry(parent_pid)
            .or_default()
            .push(child_pid);
    }

    /// Get the completed hierarchy
    pub fn into_hierarchy(self) -> ProcessHierarchy {
        self.hierarchy
    }

    /// Get a reference to the current hierarchy
    pub fn hierarchy(&self) -> &ProcessHierarchy {
        &self.hierarchy
    }
}
