use anyhow::Context;
use exectrack::{HierarchyBuilder, Tracker};
use std::process::Command;

/// Helper to track a command and build its process hierarchy
pub fn track_command(
    command: &str,
    args: &[&str],
) -> anyhow::Result<(
    runner_shared::artifacts::ProcessHierarchy,
    std::thread::JoinHandle<()>,
)> {
    // Create tracker FIRST, before spawning the child
    let mut tracker = Tracker::new()?;

    // Now spawn the child
    let mut child = Command::new(command)
        .args(args)
        .spawn()
        .context("Failed to spawn command")?;
    let root_pid = child.id() as i32;

    // Track the child process
    let rx = tracker.track(root_pid)?;

    // Build hierarchy from events in a separate thread (like the CLI does)
    let hierarchy_thread = std::thread::spawn(move || {
        let mut builder = HierarchyBuilder::new(root_pid);
        for event in rx {
            builder.process_event(&event);
        }
        builder.into_hierarchy()
    });

    // Wait for child to complete
    let _ = child.wait()?;

    // Drop tracker to close the event channel
    drop(tracker);

    // Get the hierarchy
    let hierarchy = hierarchy_thread.join().unwrap();

    eprintln!("Tracked {} processes", hierarchy.processes.len());

    // Return a dummy thread handle
    let thread_handle = std::thread::spawn(|| {});

    Ok((hierarchy, thread_handle))
}

// ============================================================================
// INTEGRATION TESTS - CHILD PROCESS TRACKING
// ============================================================================

/// Test that a single process (no children) is tracked correctly
#[test_log::test]
fn test_single_process_no_children() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("sleep", &["1"])?;

    // Should have the root process
    assert!(
        hierarchy.processes.contains_key(&hierarchy.root_pid),
        "Root process should be tracked"
    );

    // Should have no children
    assert!(
        hierarchy.children.is_empty(),
        "Single process should have no children"
    );

    thread_handle.join().unwrap();
    Ok(())
}

/// Test that bash spawning a single child process is tracked
#[test_log::test]
fn test_bash_single_child() -> anyhow::Result<()> {
    // Use a subshell to force a fork
    let (hierarchy, thread_handle) = track_command("bash", &["-c", "(sleep 0.5)"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Should have at least 2 processes (bash + sleep or bash + subshell)
    assert!(
        hierarchy.processes.len() >= 2,
        "Expected at least 2 processes, got {}",
        hierarchy.processes.len()
    );

    // Should have parent-child relationships
    assert!(
        !hierarchy.children.is_empty(),
        "Expected parent-child relationships to be tracked"
    );

    thread_handle.join().unwrap();
    Ok(())
}

/// Test that bash spawning multiple children is tracked
#[test_log::test]
fn test_bash_multiple_children() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("bash", &["-c", "sleep 0.5 & sleep 1"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Should have at least 2 processes (bash + at least one sleep)
    // Note: May not capture all children due to timing
    assert!(
        hierarchy.processes.len() >= 2,
        "Expected at least 2 processes (bash + children), got {}",
        hierarchy.processes.len()
    );

    // Should have parent-child relationships
    assert!(
        !hierarchy.children.is_empty(),
        "Expected parent-child relationships to be tracked"
    );

    // Find the bash process
    let bash_pids: Vec<_> = hierarchy
        .processes
        .iter()
        .filter(|(_, meta)| meta.name.contains("bash"))
        .map(|(pid, _)| pid)
        .collect();

    assert!(
        !bash_pids.is_empty(),
        "Expected to find bash process in hierarchy"
    );

    // Check that bash has at least 1 child
    for bash_pid in bash_pids {
        if let Some(children) = hierarchy.children.get(bash_pid) {
            eprintln!("Bash PID {} has {} children", bash_pid, children.len());
            if !children.is_empty() {
                // Found the parent with children
                thread_handle.join().unwrap();
                return Ok(());
            }
        }
    }

    panic!("Expected bash to have at least 1 child");
}

/// Test nested process hierarchy (bash -> bash -> sleep)
#[test_log::test]
fn test_nested_process_hierarchy() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("bash", &["-c", "bash -c '(sleep 0.5)'"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Should have at least 2 processes (may have exec optimization)
    assert!(
        hierarchy.processes.len() >= 2,
        "Expected at least 2 processes, got {}",
        hierarchy.processes.len()
    );

    // Should have parent-child relationships
    assert!(
        !hierarchy.children.is_empty(),
        "Expected parent-child relationships to be tracked"
    );

    thread_handle.join().unwrap();
    Ok(())
}

/// Test that exit codes are captured
#[test_log::test]
fn test_exit_code_capture() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("bash", &["-c", "exit 42"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Find any process with an exit code
    let has_exit_code = hierarchy
        .processes
        .values()
        .any(|meta| meta.exit_code.is_some());

    assert!(
        has_exit_code,
        "Expected at least one process to have an exit code"
    );

    thread_handle.join().unwrap();
    Ok(())
}

/// Test with sh instead of bash
#[test_log::test]
fn test_sh_with_children() -> anyhow::Result<()> {
    // Use subshell to force fork
    let (hierarchy, thread_handle) = track_command("sh", &["-c", "(sleep 0.5)"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Should have at least 2 processes (sh + sleep or subshell)
    assert!(
        hierarchy.processes.len() >= 2,
        "Expected at least 2 processes (sh + sleep), got {}",
        hierarchy.processes.len()
    );

    thread_handle.join().unwrap();
    Ok(())
}

/// Test process names are captured correctly
#[test_log::test]
fn test_process_names_captured() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("sleep", &["0.5"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Should find processes with expected names
    let has_sleep = hierarchy
        .processes
        .values()
        .any(|meta| meta.name == "sleep");

    assert!(has_sleep, "Expected to find 'sleep' process in hierarchy");

    thread_handle.join().unwrap();
    Ok(())
}

/// Test that start and stop times are recorded
#[test_log::test]
fn test_timestamps_recorded() -> anyhow::Result<()> {
    let (hierarchy, thread_handle) = track_command("sleep", &["0.1"])?;

    eprintln!("Hierarchy: {hierarchy:#?}");

    // Check that processes have timestamps
    for (pid, meta) in &hierarchy.processes {
        eprintln!("PID {} has start time: {} ns", pid, meta.start_time);
        // Start time should be non-zero (nanoseconds since epoch)
        assert!(
            meta.start_time > 0,
            "Process {} should have valid start time",
            pid
        );
    }

    thread_handle.join().unwrap();
    Ok(())
}
