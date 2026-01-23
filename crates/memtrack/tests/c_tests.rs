mod shared;

use memtrack::EventType;
use rstest::rstest;
use runner_shared::artifacts::MemtrackEventKind;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Compiles C source code and returns the binary path
fn compile_c_source(
    source_code: &str,
    name: &str,
    output_dir: &Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let source_path = output_dir.join(format!("{name}.c"));
    let binary_path = output_dir.join(name);

    fs::write(&source_path, source_code)?;

    let output = Command::new("gcc")
        .args(["-o", binary_path.to_str().unwrap()])
        .arg(&source_path)
        .output()?;

    if !output.status.success() {
        eprintln!("gcc stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to compile C fixture".into());
    }

    Ok(binary_path)
}

// ============================================================================
// PARAMETERIZED ALLOCATION TESTS
// ============================================================================

/// Test case definition for allocation tracking tests
struct AllocationTestCase {
    name: &'static str,
    source: &'static str,
    assertions: &'static [(EventType, usize)],
    allow_excess: bool, // Whether to allow >= instead of exact == for expected counts
}

const ALLOCATION_TEST_CASES: &[AllocationTestCase] = &[
    AllocationTestCase {
        name: "double_malloc",
        source: include_str!("../testdata/double_malloc.c"),
        assertions: &[(EventType::Malloc, 2)],
        allow_excess: false,
    },
    AllocationTestCase {
        name: "malloc_free",
        source: include_str!("../testdata/malloc_free.c"),
        assertions: &[(EventType::Malloc, 1), (EventType::Free, 1)],
        allow_excess: false,
    },
    AllocationTestCase {
        name: "calloc_test",
        source: include_str!("../testdata/calloc_test.c"),
        assertions: &[(EventType::Calloc, 1), (EventType::Free, 1)],
        allow_excess: false,
    },
    AllocationTestCase {
        name: "realloc_test",
        source: include_str!("../testdata/realloc_test.c"),
        assertions: &[
            (EventType::Malloc, 1),
            (EventType::Realloc, 1),
            (EventType::Free, 1),
        ],
        allow_excess: false,
    },
    AllocationTestCase {
        name: "aligned_alloc_test",
        source: include_str!("../testdata/aligned_alloc_test.c"),
        assertions: &[(EventType::AlignedAlloc, 1), (EventType::Free, 1)],
        allow_excess: false,
    },
    AllocationTestCase {
        name: "many_allocs",
        source: include_str!("../testdata/many_allocs.c"),
        assertions: &[(EventType::Malloc, 100), (EventType::Free, 100)],
        allow_excess: true, // Allow >= because we allocate ptrs array + 100 allocations
    },
];

#[rstest]
#[case(&ALLOCATION_TEST_CASES[0])]
#[case(&ALLOCATION_TEST_CASES[1])]
#[case(&ALLOCATION_TEST_CASES[2])]
#[case(&ALLOCATION_TEST_CASES[3])]
#[case(&ALLOCATION_TEST_CASES[4])]
#[case(&ALLOCATION_TEST_CASES[5])]
#[test_log::test]
fn test_allocation_tracking(
    #[case] test_case: &AllocationTestCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let binary = compile_c_source(test_case.source, test_case.name, temp_dir.path())?;

    let (events, thread_handle) = shared::track_binary(&binary)?;

    for (event_type, expected_count) in test_case.assertions {
        let actual_count = shared::count_events_by_type(&events, *event_type);

        if test_case.allow_excess {
            assert!(
                actual_count >= *expected_count,
                "Test '{}': Expected at least {} {:?} events, got {}",
                test_case.name,
                expected_count,
                event_type,
                actual_count
            );
        } else {
            assert_eq!(
                actual_count, *expected_count,
                "Test '{}': Expected {} {:?} events, got {}",
                test_case.name, expected_count, event_type, actual_count
            );
        }
    }

    thread_handle.join().unwrap();

    Ok(())
}

#[test]
fn test_fork_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source = include_str!("../testdata/fork_test.c");
    let binary = compile_c_source(source, "fork_test", temp_dir.path())?;

    let (events, thread_handle) = shared::track_binary(&binary)?;

    let malloc_count = shared::count_events_by_type(&events, EventType::Malloc);
    let free_count = shared::count_events_by_type(&events, EventType::Free);

    // Should have at least 2 mallocs (parent + child)
    assert!(
        malloc_count >= 2,
        "Expected at least 2 malloc events (parent + child), got {malloc_count}"
    );

    // Should have at least 2 frees (parent + child)
    assert!(
        free_count >= 2,
        "Expected at least 2 free events (parent + child), got {free_count}"
    );

    // Verify we have events from different PIDs (parent and child)
    let pids: std::collections::HashSet<u32> = events.iter().map(|e| e.pid as u32).collect();
    assert!(
        !pids.is_empty(),
        "Expected to track at least parent process"
    );

    thread_handle.join().unwrap();

    Ok(())
}

#[test]
fn test_allocation_sizes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source = include_str!("../testdata/alloc_size.c");
    let binary = compile_c_source(source, "alloc_size", temp_dir.path())?;

    let (events, thread_handle) = shared::track_binary(&binary)?;

    // Filter malloc events and collect their sizes
    let malloc_events: Vec<u64> = events
        .iter()
        .filter_map(|e| match e.kind {
            MemtrackEventKind::Malloc { size } => Some(size),
            _ => None,
        })
        .collect();

    // Expected sizes from alloc_size.c: 1024, 2048, 512, 4096
    let expected_sizes = vec![1024u64, 2048, 512, 4096];

    // Check that we have exactly 4 malloc events
    assert_eq!(
        malloc_events.len(),
        4,
        "Expected 4 malloc events, got {}",
        malloc_events.len()
    );

    // Check that all expected sizes are present
    for expected_size in &expected_sizes {
        assert!(
            malloc_events.contains(expected_size),
            "Expected allocation size {expected_size} not found in malloc events: {malloc_events:?}"
        );
    }

    // Check that we have 4 free events
    let free_count = shared::count_events_by_type(&events, EventType::Free);
    assert_eq!(free_count, 4, "Expected 4 free events, got {free_count}");

    thread_handle.join().unwrap();

    Ok(())
}
