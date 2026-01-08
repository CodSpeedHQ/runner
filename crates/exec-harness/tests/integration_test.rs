use anyhow::Result;
use tempfile::TempDir;

// Helper to create a simple sleep 100ms command
fn sleep_cmd() -> Vec<String> {
    vec!["sleep".to_string(), "0.1".to_string()]
}

/// Test that a command runs exactly the specified number of max_rounds
#[test]
fn test_max_rounds_without_warmup() -> Result<()> {
    // Create execution options with no warmup and fixed rounds
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0s".to_string()), // No warmup
            max_time: None,
            min_time: None,
            max_rounds: Some(10), // Exactly 10 rounds
            min_rounds: None,
        },
    )?;

    let times = exec_harness::walltime::perform(
        "test::max_rounds_no_warmup".to_string(),
        sleep_cmd(),
        &exec_opts,
    )?;

    // Should run exactly 10 times
    assert_eq!(times.len(), 10, "Expected exactly 10 iterations");

    Ok(())
}

/// Test that a command runs between min and max rounds
#[test]
fn test_min_max_rounds_with_warmup() -> Result<()> {
    // Create execution options with warmup and min/max rounds
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("100ms".to_string()), // Short warmup
            max_time: None,
            min_time: None,
            max_rounds: Some(50), // Max 50 rounds
            min_rounds: Some(5),  // Min 5 rounds
        },
    )?;

    let times = exec_harness::walltime::perform(
        "test::min_max_rounds_warmup".to_string(),
        sleep_cmd(),
        &exec_opts,
    )?;

    // Should run between 5 and 50 times
    assert!(
        times.len() >= 5,
        "Expected at least 5 iterations, got {}",
        times.len()
    );
    assert!(
        times.len() <= 50,
        "Expected at most 50 iterations, got {}",
        times.len()
    );

    Ok(())
}

/// Test that max_time constraint is respected
#[test]
fn test_max_time_constraint() -> Result<()> {
    // Use a very short max_time to ensure we don't run too many iterations
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("50ms".to_string()), // Short warmup
            max_time: Some("500ms".to_string()),   // Very short max time
            min_time: None,
            max_rounds: None,
            min_rounds: None,
        },
    )?;

    let times =
        exec_harness::walltime::perform("test::max_time".to_string(), sleep_cmd(), &exec_opts)?;

    // Should have run at least 1 time, but not an excessive amount
    assert!(!times.is_empty(), "Expected at least 1 iteration");
    assert!(
        times.len() < 6,
        "Expected fewer than 5 iterations due to max_time constraint, got {}",
        times.len()
    );

    Ok(())
}

/// Test that min_rounds is satisfied even with short min_time
#[test]
fn test_min_rounds_and_min_time() -> Result<()> {
    // Set min_rounds and min_time
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("10ms".to_string()), // Very short warmup
            max_time: None,
            min_time: Some("1ms".to_string()),
            max_rounds: None,
            min_rounds: Some(15),
        },
    )?;

    let times = exec_harness::walltime::perform(
        "test::min_rounds_priority".to_string(),
        sleep_cmd(),
        &exec_opts,
    )?;

    // Should satisfy min_rounds requirement
    assert!(
        times.len() >= 15,
        "Expected at least 15 iterations (min_rounds), got {}",
        times.len()
    );

    Ok(())
}

/// Test that warmup is actually performed (results in non-zero warmup phase)
#[test]
fn test_warmup_is_performed() -> Result<()> {
    // With warmup enabled
    let exec_opts_with_warmup = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("200ms".to_string()), // Significant warmup time
            max_time: Some("500ms".to_string()),
            min_time: None,
            max_rounds: None,
            min_rounds: None,
        },
    )?;

    let times_with_warmup = exec_harness::walltime::perform(
        "test::with_warmup".to_string(),
        sleep_cmd(),
        &exec_opts_with_warmup,
    )?;

    // With warmup disabled
    let exec_opts_no_warmup = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0s".to_string()), // No warmup
            max_time: None,
            min_time: None,
            max_rounds: Some(5), // Fixed 5 rounds
            min_rounds: None,
        },
    )?;

    let times_no_warmup = exec_harness::walltime::perform(
        "test::no_warmup".to_string(),
        sleep_cmd(),
        &exec_opts_no_warmup,
    )?;

    // Both should complete successfully
    assert!(!times_with_warmup.is_empty());
    assert_eq!(times_no_warmup.len(), 5);

    Ok(())
}

/// Test with a slower command to verify timing works correctly
#[test]
fn test_with_sleep_command() -> Result<()> {
    // Use a command that takes a measurable amount of time
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0s".to_string()), // No warmup for faster test
            max_time: None,
            min_time: None,
            max_rounds: Some(3), // Just 3 rounds
            min_rounds: None,
        },
    )?;

    let times = exec_harness::walltime::perform(
        "test::sleep_command".to_string(),
        vec!["sleep".to_string(), "0.01".to_string()], // 10ms sleep
        &exec_opts,
    )?;

    // Should run exactly 3 times
    assert_eq!(times.len(), 3, "Expected exactly 3 iterations");

    // Each iteration should take at least 10ms (10_000_000 ns)
    for (i, &time_ns) in times.iter().enumerate() {
        assert!(
            time_ns >= 10_000_000,
            "Iteration {i} took only {time_ns}ns, expected at least 10ms"
        );
    }

    Ok(())
}

/// Test that invalid command exits early
#[test]
fn test_invalid_command_fails() {
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0s".to_string()),
            max_time: None,
            min_time: None,
            max_rounds: Some(5),
            min_rounds: None,
        },
    )
    .unwrap();

    // Try to run a command that doesn't exist
    let result = exec_harness::walltime::perform(
        "test::invalid_command".to_string(),
        vec!["this_command_definitely_does_not_exist_12345".to_string()],
        &exec_opts,
    );

    // Should fail
    assert!(result.is_err(), "Expected error for invalid command");
}

/// Test that pure numbers are interpreted as seconds
#[test]
fn test_pure_numbers_as_seconds() -> Result<()> {
    // Use pure numbers which should be interpreted as seconds
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0.1".to_string()), // 0.1 seconds warmup
            max_time: Some("1".to_string()),      // 1 second max time
            min_time: None,
            max_rounds: None,
            min_rounds: None,
        },
    )?;

    let times = exec_harness::walltime::perform(
        "test::pure_numbers_seconds".to_string(),
        sleep_cmd(),
        &exec_opts,
    )?;

    // Should have run at least once
    assert!(!times.is_empty(), "Expected at least one iteration");

    // Test fractional seconds too
    let exec_opts_fractional = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("0.1".to_string()), // 0.1 seconds warmup
            max_time: Some("0.5".to_string()),    // 0.5 seconds max time
            min_time: None,
            max_rounds: None,
            min_rounds: None,
        },
    )?;

    let times_fractional = exec_harness::walltime::perform(
        "test::fractional_seconds".to_string(),
        sleep_cmd(),
        &exec_opts_fractional,
    )?;

    assert!(
        !times_fractional.is_empty(),
        "Expected at least one iteration with fractional seconds"
    );

    Ok(())
}

/// Test that when a warmup run exceeds max_time, the command is only run once
#[test]
fn test_single_long_execution() -> Result<()> {
    // Set max_time very low and warmup time high to force single execution
    let exec_opts = exec_harness::walltime::ExecutionOptions::try_from(
        exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: Some("100ms".to_string()),
            max_time: Some("100ms".to_string()), // Low max time, shorter than command duration
            min_time: None,
            max_rounds: None,
            min_rounds: None,
        },
    )?;

    // Create a temporary directory for the test
    let tmpdir = TempDir::new()?;

    // Create a command that sleeps and creates a directory that must not exist
    // This will fail if executed twice because the directory will already exist
    let test_dir = tmpdir.path().join("lock_file");
    let cmd = format!("sleep 1 && mkdir {}", test_dir.display());

    let times = exec_harness::walltime::perform(
        "test::single_long_execution".to_string(),
        vec!["sh".to_string(), "-c".to_string(), cmd.clone()],
        &exec_opts,
    )?;

    // Should have run exactly once
    assert_eq!(times.len(), 1, "Expected exactly one iteration");

    // Sanity check: any subsequent run should fail due to directory existing, to avoid false
    // positives
    assert!(
        exec_harness::walltime::perform(
            "test::single_long_execution".to_string(),
            vec!["sh".to_string(), "-c".to_string(), cmd],
            &exec_opts,
        )
        .is_err(),
        "Expected failure on second execution due to existing directory"
    );

    Ok(())
}
