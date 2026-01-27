use super::EXEC_HARNESS_COMMAND;
use crate::prelude::*;
use crate::project_config::Target;
use crate::project_config::WalltimeOptions;
use exec_harness::BenchmarkCommand;

/// Convert targets from project config to exec-harness JSON input format
pub fn targets_to_exec_harness_json(
    targets: &[Target],
    default_walltime: Option<&WalltimeOptions>,
) -> Result<String> {
    let inputs: Vec<BenchmarkCommand> = targets
        .iter()
        .map(|target| {
            // Parse the exec string into command parts
            let command = shell_words::split(&target.exec)
                .with_context(|| format!("Failed to parse command: {}", target.exec))?;

            // Merge target-specific walltime options with defaults
            let target_walltime = target.options.as_ref().and_then(|o| o.walltime.as_ref());
            let walltime_args = merge_walltime_options(default_walltime, target_walltime);

            Ok(BenchmarkCommand {
                command,
                name: target.name.clone(),
                walltime_args,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    serde_json::to_string(&inputs).context("Failed to serialize targets to JSON")
}

/// Merge default walltime options with target-specific overrides
fn merge_walltime_options(
    default: Option<&WalltimeOptions>,
    target: Option<&WalltimeOptions>,
) -> exec_harness::walltime::WalltimeExecutionArgs {
    let default_args = default.map(walltime_options_to_args);
    let target_args = target.map(walltime_options_to_args);

    match (default_args, target_args) {
        (None, None) => exec_harness::walltime::WalltimeExecutionArgs::default(),
        (Some(d), None) => d,
        (None, Some(t)) => t,
        (Some(d), Some(t)) => exec_harness::walltime::WalltimeExecutionArgs {
            warmup_time: t.warmup_time.or(d.warmup_time),
            max_time: t.max_time.or(d.max_time),
            min_time: t.min_time.or(d.min_time),
            max_rounds: t.max_rounds.or(d.max_rounds),
            min_rounds: t.min_rounds.or(d.min_rounds),
        },
    }
}

/// Convert project config WalltimeOptions to exec-harness WalltimeExecutionArgs
fn walltime_options_to_args(
    opts: &WalltimeOptions,
) -> exec_harness::walltime::WalltimeExecutionArgs {
    exec_harness::walltime::WalltimeExecutionArgs {
        warmup_time: opts.warmup_time.clone(),
        max_time: opts.max_time.clone(),
        min_time: opts.min_time.clone(),
        max_rounds: opts.max_rounds,
        min_rounds: opts.min_rounds,
    }
}

/// Build a command that pipes targets JSON to exec-harness via stdin
pub fn build_pipe_command(
    targets: &[Target],
    default_walltime: Option<&WalltimeOptions>,
) -> Result<Vec<String>> {
    let json = targets_to_exec_harness_json(targets, default_walltime)?;
    // Use a heredoc to safely pass the JSON to exec-harness
    Ok(vec![
        EXEC_HARNESS_COMMAND.to_owned(),
        "-".to_owned(),
        "<<".to_owned(),
        "'CODSPEED_EOF'\n".to_owned(),
        json,
        "\nCODSPEED_EOF".to_owned(),
    ])
}
