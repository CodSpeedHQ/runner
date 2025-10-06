use async_trait::async_trait;

use crate::prelude::*;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};

use super::setup::install_valgrind;
use super::{helpers::perf_maps::harvest_perf_maps, helpers::venv_compat, measure};

pub struct ValgrindExecutor;

#[async_trait(?Send)]
impl Executor for ValgrindExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::Valgrind
    }

    async fn setup(&self, system_info: &SystemInfo) -> Result<()> {
        // Valgrind / Callgrind is not supported on macOS (notably arm64 macOS).
        // Instead of failing fast, allow the executor to run but skip installing
        // Valgrind. The measure implementation contains a macOS fallback that
        // runs the benchmark without instrumentation so users can still run
        // benchmarks locally on macOS.
        if cfg!(target_os = "macos") {
            warn!(
                "Valgrind/Callgrind is not supported on macOS: skipping Valgrind installation. Benchmarks will run without instrumentation."
            );
        } else {
            install_valgrind(system_info).await?;
        }

        if let Err(error) = venv_compat::symlink_libpython(None) {
            warn!("Failed to symlink libpython");
            debug!("Script error: {error}");
        }

        Ok(())
    }

    async fn run(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
        mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        // On macOS, callgrind is not available. Let the measure function handle
        // the macOS fallback (it will run the benchmark without instrumentation)
        // so users can still run benchmarks locally. On non-macOS platforms we
        // proceed with the regular Valgrind-based instrumentation.
        // TODO: add valgrind version check for non-macOS platforms
        if cfg!(target_os = "macos") {
            info!(
                "Running Valgrind executor on macOS: benchmarks will run without Callgrind instrumentation."
            );
        }
        measure::measure(config, &run_data.profile_folder, mongo_tracer).await?;

        Ok(())
    }

    async fn teardown(
        &self,
        _config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        harvest_perf_maps(&run_data.profile_folder).await?;

        Ok(())
    }
}
