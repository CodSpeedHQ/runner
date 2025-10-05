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

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        "''".to_string()
    } else if !value.contains('\'') {
        format!("'{value}'")
    } else {
        let mut quoted = String::with_capacity(value.len() + 2);
        quoted.push('\'');
        for (idx, part) in value.split('\'').enumerate() {
            if idx > 0 {
                quoted.push_str("'\"'\"'");
            }
            quoted.push_str(part);
        }
        quoted.push('\'');
        quoted
    }
}

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

        let use_systemd = cfg!(target_os = "linux")
            && std::process::Command::new("which")
                .arg("systemd-run")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        let base_injected_env =
            get_base_injected_env(RunnerMode::Walltime, &run_data.profile_folder)
                .into_iter()
                .map(|(k, v)| format!("export {k}={}", shell_quote(&v)))
                .collect::<Vec<_>>()
                .join("\n");

        let path_env = std::env::var("PATH").unwrap_or_default();
        let path_value = format!(
            "{}:{}:{}",
            introspected_nodejs::setup()
                .map_err(|e| anyhow!("failed to setup NodeJS introspection. {e}"))?
                .to_string_lossy(),
            introspected_golang::setup()
                .map_err(|e| anyhow!("failed to setup Go introspection. {e}"))?
                .to_string_lossy(),
            path_env
        );
        let path_env = format!("export PATH={}", shell_quote(&path_value));

        let combined_env = if use_systemd {
            let system_env = get_exported_system_env()?;
            // Sanitize system `export` output to only include valid shell identifiers.
            // Some environments (editor integrations, IDEs) can inject nonsense names
            // which `bash` rejects when `source`-ing the env file. Filter them out.
            let mut sanitized = String::new();
            for line in system_env.lines() {
                // strip leading `declare -x ` if present (bash `export` prints that form)
                let mut l = line.trim();
                if let Some(rest) = l.strip_prefix("declare -x ") {
                    l = rest.trim();
                }
                if let Some(rest) = l.strip_prefix("export ") {
                    l = rest.trim();
                }

                // left of '=' is the var name; ensure it's a valid identifier: [A-Za-z_][A-Za-z0-9_]*
                if let Some(eq) = l.find('=') {
                    let name = &l[..eq].trim();
                    let mut chars = name.chars();
                    if let Some(first) = chars.next() {
                        if (first == '_' || first.is_ascii_alphabetic())
                            && chars.clone().all(|c| c == '_' || c.is_ascii_alphanumeric())
                        {
                            sanitized.push_str(line);
                            sanitized.push('\n');
                        }
                    }
                }
            }

            format!("{sanitized}{base_injected_env}\n{path_env}")
        } else {
            format!("{base_injected_env}\n{path_env}")
        };

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
        // Prefer using systemd-run on Linux hosts when available since it
        // provides the `--scope` isolation we need. On macOS (or when
        // systemd isn't installed) fall back to invoking `bash <script>`
        // directly.
        let cmd = if use_systemd {
            format!(
                "systemd-run {quiet_flag} --scope --slice=codspeed.slice --same-dir --uid={uid} --gid={gid} -- bash {}",
                script_file.path().display()
            )
        } else {
            // Directly invoke bash with the script path. The command will
            // be executed via `sh -c '<bench_cmd>'` or `sudo sh -c '<bench_cmd>'`.
            format!("bash {}", script_file.path().display())
        };
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
        // Note: (jzombie) Workaround for asking for `sudo password` on macOS.
        // Only use sudo when perf is enabled and available; otherwise run the
        // benchmark directly to avoid prompting for passwords on CI (macOS
        // runners, etc.).
        // TODO: There is also a `run_with_sudo` in `setup.rs` that might be a
        // better way to approach this
        let use_sudo = self.perf.is_some() && config.enable_perf;
        // If we need sudo, the command will be `sudo sh -c '<bench_cmd>'`.
        // Otherwise we invoke the shell directly as `sh -c '<bench_cmd>'`.
        let mut cmd = if use_sudo {
            Command::new("sudo")
        } else {
            Command::new("sh")
        };

        let effective_cwd = if let Some(cwd) = &config.working_directory {
            canonicalize(cwd)?
        } else {
            std::env::current_dir().context("failed to determine current working directory")?
        };
        cmd.current_dir(&effective_cwd);
        // Ensure the spawned shell inherits the same PWD. Some shells rely on the
        // PWD env var instead of calling getcwd(), so set it explicitly.
        cmd.env("PWD", &effective_cwd);

        let debug_enabled = is_codspeed_debug_enabled();
        if debug_enabled {
            debug!(
                "Effective bench working directory: {}",
                effective_cwd.display()
            );
        }

        let target_root = match std::env::var("CARGO_TARGET_DIR") {
            Ok(dir) => {
                let path = PathBuf::from(&dir);
                if path.is_absolute() {
                    if debug_enabled {
                        debug!("CARGO_TARGET_DIR env: {}", path.display());
                    }
                    path
                } else {
                    let abs = effective_cwd.join(&path);
                    if debug_enabled {
                        debug!(
                            "CARGO_TARGET_DIR env (relative): {} -> {}",
                            path.display(),
                            abs.display()
                        );
                    }
                    abs
                }
            }
            Err(_) => {
                let default = effective_cwd.join("target");
                if debug_enabled {
                    debug!(
                        "CARGO_TARGET_DIR not set; defaulting to {}",
                        default.display()
                    );
                }
                default
            }
        };

        if debug_enabled {
            let release_deps = target_root.join("release").join("deps");
            if release_deps.exists() {
                if let Ok(entries) = std::fs::read_dir(&release_deps) {
                    debug!("Listing {} (first 50 entries)", release_deps.display());
                    for e in entries.flatten().take(50) {
                        if let Ok(md) = e.metadata() {
                            debug!(" - {} (len: {})", e.path().display(), md.len());
                        }
                    }
                }
            } else {
                debug!("Release deps directory missing: {}", release_deps.display());
            }
        }

        if cfg!(target_os = "macos") {
            Self::ensure_walltime_bench_artifacts(config, &effective_cwd, &target_root)?;
        }

        let status = {
            let _guard = HookScriptsGuard::setup();

            let (_env_file, _script_file, bench_cmd) = Self::walltime_bench_cmd(config, run_data)?;
            if let Some(perf) = &self.perf
                && config.enable_perf
            {
                // Perf runner expects the `cmd` to be either `sudo` (so that
                // it becomes `sudo sh -c ...`) or `sh` (in which case the
                // perf runner will arrange execution itself). Pass through.
                perf.run(cmd, &bench_cmd, config).await
            } else {
                // Add the appropriate arguments depending on whether we're
                // invoking via sudo or directly. When using sudo, the args
                // must be: `sh -c '<bench_cmd>'`. When not using sudo, the
                // args are just `-c '<bench_cmd>'` because the program is
                // already `sh`.
                if use_sudo {
                    cmd.args(["sh", "-c", &bench_cmd]);
                } else {
                    cmd.args(["-c", &bench_cmd]);
                }
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

impl WallTimeExecutor {
    fn ensure_walltime_bench_artifacts(
        config: &Config,
        effective_cwd: &Path,
        target_root: &Path,
    ) -> Result<()> {
        let codspeed_dir = target_root.join("codspeed");
        let walltime_dir = codspeed_dir.join("walltime");
        if has_any_files(&walltime_dir)? {
            return Ok(());
        }

        let instrumentation_dir = codspeed_dir.join("instrumentation");
        if has_any_files(&instrumentation_dir)? {
            info!(
                "No walltime CodSpeed artifacts found; instrumentation artifacts detected. Rebuilding benchmarks in walltime mode."
            );
        } else {
            info!(
                "No CodSpeed walltime artifacts detected; running `cargo codspeed build --measurement-mode walltime`."
            );
        }

        let mut args = vec!["codspeed", "build", "--measurement-mode", "walltime"];
        if config.command.contains("--workspace") {
            args.push("--workspace");
        }

        let status = Command::new("cargo")
            .current_dir(effective_cwd)
            .env("CODSPEED_RUNNER_MODE", "walltime")
            .args(&args)
            .status()
            .context("failed to invoke `cargo codspeed build --measurement-mode walltime`")?;

        if !status.success() {
            bail!("`cargo codspeed build --measurement-mode walltime` exited with status {status}");
        }

        if !has_any_files(&walltime_dir)? {
            bail!(
                "`cargo codspeed build --measurement-mode walltime` completed but no walltime artifacts were produced"
            );
        }

        Ok(())
    }
}

fn has_any_files(path: &Path) -> Result<bool> {
    let Ok(read_dir) = std::fs::read_dir(path) else {
        return Ok(false);
    };

    for entry in read_dir.flatten() {
        if entry.path().is_file() {
            return Ok(true);
        }
        if entry.path().is_dir() {
            // If the directory contains anything (recursively), treat it as non-empty.
            if has_any_files(&entry.path())? {
                return Ok(true);
            }
        }
    }

    Ok(false)
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
