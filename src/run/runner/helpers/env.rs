use std::{collections::HashMap, env::consts::ARCH, path::Path};

use crate::run::runner::RunnerMode;

pub fn get_base_injected_env(
    mode: RunnerMode,
    profile_folder: &Path,
) -> HashMap<&'static str, String> {
    HashMap::from([
        ("PYTHONHASHSEED", "0".into()),
        (
            "PYTHON_PERF_JIT_SUPPORT",
            if mode == RunnerMode::Walltime {
                "1".into()
            } else {
                "0".into()
            },
        ),
        ("ARCH", ARCH.into()),
        ("CODSPEED_ENV", "runner".into()),
        ("CODSPEED_RUNNER_MODE", mode.to_string()),
        (
            "CODSPEED_PROFILE_FOLDER",
            profile_folder.to_string_lossy().to_string(),
        ),
    ])
}

pub fn is_codspeed_debug_enabled() -> bool {
    let log_level = std::env::var("CODSPEED_LOG")
        .ok()
        .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Info);

    log_level < log::LevelFilter::Debug
}
