use super::perf::PerfRunner;
use crate::prelude::*;
use crate::run::RunnerMode;
use crate::run::instruments::mongo_tracer::MongoTracer;
use crate::run::runner::executor::Executor;
use crate::run::runner::helpers::env::{get_base_injected_env, is_codspeed_debug_enabled};
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::introspected_golang;
use crate::run::runner::helpers::introspected_nodejs;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::{ExecutorName, RunData};
use crate::run::{check_system::SystemInfo, config::Config};
use async_trait::async_trait;
use std::fs::canonicalize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

struct HookScriptsGuard {
    post_bench_script: PathBuf,
}

impl HookScriptsGuard {
    fn execute_script_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        if !path.exists() || !path.is_file() {
            debug!("Script not found or not a file: {}", path.display());
            return Ok(());
        }

        let output = Command::new("bash").args([&path]).output()?;
        if !output.status.success() {
            debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            debug!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            bail!("Failed to execute script: {}", path.display());
        }

        Ok(())
    }

    pub fn setup_with_scripts<P: AsRef<Path>>(pre_bench_script: P, post_bench_script: P) -> Self {
        if let Err(e) = Self::execute_script_from_path(pre_bench_script.as_ref()) {
            debug!("Failed to execute pre-bench script: {e}");
        }

        Self {
            post_bench_script: post_bench_script.as_ref().to_path_buf(),
        }
    }

    pub fn setup() -> Self {
        Self::setup_with_scripts(
            "/usr/local/bin/codspeed-pre-bench",
            "/usr/local/bin/codspeed-post-bench",
        )
    }
}

impl Drop for HookScriptsGuard {
    fn drop(&mut self) {
        if let Err(e) = Self::execute_script_from_path(&self.post_bench_script) {
            debug!("Failed to execute post-bench script: {e}");
        }
    }
}

/// Returns a list of exported environment variables which can be loaded with `source` in a shell.
///
/// Example: `declare -x outputs="out"`
fn get_exported_system_env() -> Result<String> {
    let output = Command::new("bash")
        .arg("-c")
        .arg("export")
        .output()
        .context("Failed to run `export`")?;
    if !output.status.success() {
        bail!(
            "Failed to get system environment variables: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).context("Failed to parse export output as UTF-8")
}

pub struct WallTimeExecutor {
    perf: Option<PerfRunner>,
}

impl WallTimeExecutor {
    pub fn new() -> Self {
        Self {
            perf: cfg!(target_os = "linux").then(PerfRunner::new),
        }
    }

    fn walltime_bench_cmd(
        config: &Config,
        run_data: &RunData,
    ) -> Result<(NamedTempFile, NamedTempFile, String)> {
        let bench_cmd = get_bench_command(config)?;

        let system_env = get_exported_system_env()?;
        let base_injected_env =
            get_base_injected_env(RunnerMode::Walltime, &run_data.profile_folder)
                .into_iter()
                .map(|(k, v)| format!("export {k}={v}",))
                .collect::<Vec<_>>()
                .join("\n");

        let path_env = std::env::var("PATH").unwrap_or_default();
        let path_env = format!(
            "export PATH={}:{}:{}",
            introspected_nodejs::setup()
                .map_err(|e| anyhow!("failed to setup NodeJS introspection. {e}"))?
                .to_string_lossy(),
            introspected_golang::setup()
                .map_err(|e| anyhow!("failed to setup Go introspection. {e}"))?
                .to_string_lossy(),
            path_env
        );

        let combined_env = format!("{system_env}\n{base_injected_env}\n{path_env}");

        let mut env_file = NamedTempFile::new()?;
        env_file.write_all(combined_env.as_bytes())?;

        let script_file = {
            let mut file = NamedTempFile::new()?;
            let bash_command = format!("source {} && {}", env_file.path().display(), bench_cmd);
            debug!("Bash command: {bash_command}");
            file.write_all(bash_command.as_bytes())?;
            file
        };

        let quiet_flag = if is_codspeed_debug_enabled() {
            "--quiet"
        } else {
            ""
        };

        // Remarks:
        // - We're using --scope so that perf is able to capture the events of the benchmark process.
        // - We can't user `--user` here because we need to run in `codspeed.slice`, otherwise we'd run in
        //   user.slice` (which is isolated). We can use `--gid` and `--uid` to run the command as the current user.
        // - We must use `bash` here instead of `sh` since `source` isn't available when symlinked to `dash`.
        // - We have to pass the environment variables because `--scope` only inherits the system and not the user environment variables.
        let uid = nix::unistd::Uid::current().as_raw();
        let gid = nix::unistd::Gid::current().as_raw();
        let cmd = format!(
            "systemd-run {quiet_flag} --scope --slice=codspeed.slice --same-dir --uid={uid} --gid={gid} -- bash {}",
            script_file.path().display()
        );
        Ok((env_file, script_file, cmd))
    }
}

#[async_trait(?Send)]
impl Executor for WallTimeExecutor {
    fn name(&self) -> ExecutorName {
        ExecutorName::WallTime
    }

    async fn setup(&self, _system_info: &SystemInfo) -> Result<()> {
        if self.perf.is_some() {
            PerfRunner::setup_environment()?;
        }

        Ok(())
    }

    async fn run(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
        _mongo_tracer: &Option<MongoTracer>,
    ) -> Result<()> {
        let mut cmd = Command::new("sudo");

        if let Some(cwd) = &config.working_directory {
            let abs_cwd = canonicalize(cwd)?;
            cmd.current_dir(abs_cwd);
        }

        let status = {
            let _guard = HookScriptsGuard::setup();

            let (_env_file, _script_file, bench_cmd) = Self::walltime_bench_cmd(config, run_data)?;
            if let Some(perf) = &self.perf
                && config.enable_perf
            {
                perf.run(cmd, &bench_cmd, config).await
            } else {
                cmd.args(["sh", "-c", &bench_cmd]);
                debug!("cmd: {cmd:?}");

                run_command_with_log_pipe(cmd).await
            }
        };

        let status = status.map_err(|e| anyhow!("failed to execute the benchmark process. {e}"))?;
        debug!("cmd exit status: {:?}", status);

        if !status.success() {
            bail!("failed to execute the benchmark process: {status}");
        }

        Ok(())
    }

    async fn teardown(
        &self,
        config: &Config,
        _system_info: &SystemInfo,
        run_data: &RunData,
    ) -> Result<()> {
        debug!("Copying files to the profile folder");

        if let Some(perf) = &self.perf
            && config.enable_perf
        {
            perf.save_files_to(&run_data.profile_folder).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use std::{
        io::{Read, Write},
        os::unix::fs::PermissionsExt,
    };

    #[test]
    fn test_env_guard_no_crash() {
        fn create_run_script(content: &str) -> anyhow::Result<NamedTempFile> {
            let rwx = std::fs::Permissions::from_mode(0o777);
            let mut script_file = tempfile::Builder::new()
                .suffix(".sh")
                .permissions(rwx)
                .keep(true)
                .tempfile()?;
            script_file.write_all(content.as_bytes())?;

            Ok(script_file)
        }

        let mut tmp_dst = tempfile::NamedTempFile::new().unwrap();

        let pre_script = create_run_script(&format!(
            "#!/usr/bin/env bash\necho \"pre\" >> {}",
            tmp_dst.path().display()
        ))
        .unwrap();
        let post_script = create_run_script(&format!(
            "#!/usr/bin/env bash\necho \"post\" >> {}",
            tmp_dst.path().display()
        ))
        .unwrap();

        {
            let _guard =
                HookScriptsGuard::setup_with_scripts(pre_script.path(), post_script.path());
        }

        let mut result = String::new();
        tmp_dst.read_to_string(&mut result).unwrap();
        assert_eq!(result, "pre\npost\n");
    }
}
