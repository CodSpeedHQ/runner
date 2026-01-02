mod config;

pub use config::ExecutionOptions;
use config::RoundOrTime;
pub use config::WalltimeExecutionArgs;
pub use runner_shared::walltime_results::WalltimeResults;

use crate::prelude::*;
use codspeed::instrument_hooks::InstrumentHooks;
use std::process::Command;

pub fn perform(
    bench_uri: String,
    command: Vec<String>,
    config: &ExecutionOptions,
) -> Result<Vec<u128>> {
    let warmup_time_ns = config.warmup_time_ns;
    let hooks = InstrumentHooks::instance();

    let do_one_round = |times_per_round_ns: &mut Vec<u128>| {
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .spawn()
            .context("Failed to execute command")?;
        let bench_round_start_ts_ns = InstrumentHooks::current_timestamp();
        let status = child
            .wait()
            .context("Failed to wait for command to finish")?;

        let bench_round_end_ts_ns = InstrumentHooks::current_timestamp();
        hooks.add_benchmark_timestamps(bench_round_start_ts_ns, bench_round_end_ts_ns);

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }

        times_per_round_ns.push((bench_round_end_ts_ns - bench_round_start_ts_ns) as u128);

        Ok(())
    };

    // Compute the number of rounds to perform, either from warmup or directly from config
    let rounds_to_perform = if warmup_time_ns > 0 {
        let mut warmup_times_ns = Vec::new();
        let warmup_start_ts_ns = InstrumentHooks::current_timestamp();

        hooks.start_benchmark().unwrap();
        while InstrumentHooks::current_timestamp() < warmup_start_ts_ns + warmup_time_ns {
            do_one_round(&mut warmup_times_ns)?;
        }
        hooks.stop_benchmark().unwrap();
        let warmup_end_ts_ns = InstrumentHooks::current_timestamp();

        if let [single_warmup_round_duration_ns] = warmup_times_ns.as_slice() {
            match config.max {
                Some(RoundOrTime::TimeNs(time_ns)) | Some(RoundOrTime::Both { time_ns, .. }) => {
                    if time_ns <= *single_warmup_round_duration_ns as u64 {
                        info!(
                            "Warmup duration ({single_warmup_round_duration_ns} ns) exceeded or met max_time ({time_ns} ns). No more rounds will be performed."
                        );
                        // Mark benchmark as executed for the runner to register
                        hooks.set_executed_benchmark(&bench_uri).unwrap();
                        return Ok(warmup_times_ns);
                    }
                }
                _ => { /* No max time constraint */ }
            }
        }

        info!("Completed {} warmup rounds", warmup_times_ns.len());

        let average_time_per_round_ns =
            (warmup_end_ts_ns - warmup_start_ts_ns) / warmup_times_ns.len() as u64;

        // Extract min rounds from config
        let actual_min_rounds = match &config.min {
            Some(RoundOrTime::Rounds(rounds)) => Some(*rounds),
            Some(RoundOrTime::TimeNs(time_ns)) => {
                Some(((time_ns + average_time_per_round_ns) / average_time_per_round_ns) + 1)
            }
            Some(RoundOrTime::Both { rounds, time_ns }) => {
                let rounds_from_time =
                    ((time_ns + average_time_per_round_ns) / average_time_per_round_ns) + 1;
                Some((*rounds).max(rounds_from_time))
            }
            None => None,
        };

        // Extract max rounds from config
        let actual_max_rounds = match &config.max {
            Some(RoundOrTime::Rounds(rounds)) => Some(*rounds),
            Some(RoundOrTime::TimeNs(time_ns)) => {
                Some((time_ns + average_time_per_round_ns) / average_time_per_round_ns)
            }
            Some(RoundOrTime::Both { rounds, time_ns }) => {
                let rounds_from_time =
                    (time_ns + average_time_per_round_ns) / average_time_per_round_ns;
                Some((*rounds).min(rounds_from_time))
            }
            None => None,
        };

        match (actual_min_rounds, actual_max_rounds) {
            (Some(min), Some(max)) if min > max => {
                warn!(
                    "Computed min rounds ({min}) is greater than max rounds ({max}). Using max rounds.",
                );
                max
            }
            (Some(min), Some(max)) => (min + max) / 2,
            (None, Some(max)) => max,
            (Some(min), None) => min,
            (None, None) => {
                bail!("Unable to determine number of rounds to perform");
            }
        }
    } else {
        // No warmup, extract rounds directly from config
        match (&config.max, &config.min) {
            (Some(RoundOrTime::Rounds(rounds)), _) | (_, Some(RoundOrTime::Rounds(rounds))) => {
                *rounds
            }
            (Some(RoundOrTime::Both { rounds, .. }), _)
            | (_, Some(RoundOrTime::Both { rounds, .. })) => *rounds,
            _ => bail!("Either max_rounds or min_rounds must be specified when warmup is disabled"),
        }
    };

    info!("Performing {rounds_to_perform} rounds");

    let round_start_ts_ns = InstrumentHooks::current_timestamp();
    let mut times_per_round_ns = Vec::with_capacity(rounds_to_perform.try_into().unwrap());

    hooks.start_benchmark().unwrap();
    for round in 0..rounds_to_perform {
        do_one_round(&mut times_per_round_ns)?;

        // Check if we've exceeded max time
        let max_time_ns = match &config.max {
            Some(RoundOrTime::TimeNs(time_ns)) | Some(RoundOrTime::Both { time_ns, .. }) => {
                Some(*time_ns)
            }
            _ => None,
        };

        if let Some(max_time_ns) = max_time_ns {
            let current_round = round + 1;
            if current_round < rounds_to_perform
                && InstrumentHooks::current_timestamp() - round_start_ts_ns > max_time_ns
            {
                info!(
                    "Prematurally reached maximum time limit after {current_round}/{rounds_to_perform} rounds, stopping here"
                );
                break;
            }
        }
    }
    hooks.stop_benchmark().unwrap();
    hooks.set_executed_benchmark(&bench_uri).unwrap();

    Ok(times_per_round_ns)
}
