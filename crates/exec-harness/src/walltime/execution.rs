use crate::prelude::*;
use codspeed::instrument_hooks::InstrumentHooks;
use std::process::Command;

#[derive(Debug)]
// TODO: Better management of defaults for this
pub(crate) struct ExecutionOptions {
    warmup_time_ns: Option<u64>,
    max_time_ns: Option<u64>,
    min_time_ns: Option<u64>,
    max_rounds: Option<u64>,
    min_rounds: Option<u64>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        const DEFAULT_MAX_TIME_NS: u64 = 3_000_000_000; // 3 seconds

        ExecutionOptions {
            warmup_time_ns: None,
            max_time_ns: Some(DEFAULT_MAX_TIME_NS),
            min_time_ns: None,
            max_rounds: None,
            min_rounds: None,
        }
    }
}

impl ExecutionOptions {
    const DEFAULT_WARMUP_TIME_NS: u64 = 1_000_000_000; // 1 second

    pub(crate) fn warmup_time_ns(&self) -> u64 {
        self.warmup_time_ns.unwrap_or(Self::DEFAULT_WARMUP_TIME_NS)
    }
}

pub(crate) fn perform(
    bench_uri: String,
    command: Vec<String>,
    config: &ExecutionOptions,
) -> Result<Vec<u128>> {
    let warmup_time_ns = config.warmup_time_ns();
    let hooks = InstrumentHooks::instance();

    let iterations_to_perform = if warmup_time_ns > 0 {
        let warmup_start_ts_ns = InstrumentHooks::current_timestamp();
        let warmup_end_ts = warmup_start_ts_ns + warmup_time_ns;
        let mut warmup_iterations = 0;

        hooks.start_benchmark().unwrap();
        while InstrumentHooks::current_timestamp() < warmup_end_ts {
            let warmup_iteration_start_ts_ns = InstrumentHooks::current_timestamp();
            let status = Command::new(&command[0])
                .args(&command[1..])
                .status()
                .context("Failed to execute command")?;
            let warmup_iteration_end_ts_ns = InstrumentHooks::current_timestamp();
            hooks
                .add_benchmark_timestamps(warmup_iteration_start_ts_ns, warmup_iteration_end_ts_ns);

            if !status.success() {
                bail!("Command exited with non-zero status: {status}");
            }

            warmup_iterations += 1;
        }
        hooks.stop_benchmark().unwrap();

        let warmup_end_ts_ns = InstrumentHooks::current_timestamp();

        debug!(
            "Completed {} warmup iterations in {} ns",
            warmup_iterations,
            warmup_end_ts_ns - warmup_start_ts_ns
        );

        // TODO: Exit here if we have already exceeded max_time_ns during warmup

        let average_time_per_iteration_ns =
            (warmup_end_ts_ns - warmup_start_ts_ns) / warmup_iterations as u64;

        let min_rounds_computed_from_warmup = config.min_time_ns.map(|min_time_ns| {
            ((min_time_ns + average_time_per_iteration_ns) / average_time_per_iteration_ns) + 1
        });

        let max_rounds_computed_from_warmup = config.max_time_ns.map(|max_time_ns| {
            (max_time_ns + average_time_per_iteration_ns) / average_time_per_iteration_ns
        });

        let actual_min_rounds = match (config.min_rounds, min_rounds_computed_from_warmup) {
            (Some(cfg_min), Some(warmup_min)) => Some(cfg_min.max(warmup_min)),
            (Some(cfg_min), None) => Some(cfg_min),
            (None, Some(warmup_min)) => Some(warmup_min),
            (None, None) => None,
        };

        let actual_max_rounds = match (config.max_rounds, max_rounds_computed_from_warmup) {
            (Some(cfg_max), Some(warmup_max)) => Some(cfg_max.min(warmup_max)),
            (Some(cfg_max), None) => Some(cfg_max),
            (None, Some(warmup_max)) => Some(warmup_max),
            (None, None) => None,
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
                bail!("Unable to determine number of iterations to perform");
            }
        }
    } else {
        config
            .max_rounds
            .or(config.min_rounds)
            .ok_or_else(|| anyhow!("Either max_rounds or min_rounds must be specified"))?
    };

    debug!("Performing {iterations_to_perform} iterations");

    let iterations_start_ts_ns = InstrumentHooks::current_timestamp();
    let mut times_per_round_ns = Vec::with_capacity(iterations_to_perform.try_into().unwrap());

    hooks.start_benchmark().unwrap();
    for iteration in 0..iterations_to_perform {
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .spawn()
            .context("Failed to execute command")?;
        let bench_iteration_start_ts_ns = InstrumentHooks::current_timestamp();
        let status = child
            .wait()
            .context("Failed to wait for command to finish")?;

        let bench_iteration_end_ts_ns = InstrumentHooks::current_timestamp();
        hooks.add_benchmark_timestamps(bench_iteration_start_ts_ns, bench_iteration_end_ts_ns);

        if !status.success() {
            bail!("Command exited with non-zero status: {status}");
        }

        times_per_round_ns.push((bench_iteration_end_ts_ns - bench_iteration_start_ts_ns) as u128);

        if let Some(max_time_ns) = config.max_time_ns {
            let current_iteration = iteration + 1;
            if current_iteration < iterations_to_perform
                && bench_iteration_end_ts_ns - iterations_start_ts_ns > max_time_ns
            {
                info!(
                    "Prematurally reached maximum time limit after {current_iteration}/{iterations_to_perform} iterations, stopping here"
                );
                break;
            }
        }
    }
    hooks.stop_benchmark().unwrap();
    hooks.set_executed_benchmark(&bench_uri).unwrap();

    Ok(times_per_round_ns)
}
