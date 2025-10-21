use crate::prelude::*;
use crate::run::runner::RunnerMode;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::introspected_golang;
use crate::run::runner::helpers::introspected_nodejs;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::valgrind::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::run::{config::Config, instruments::mongo_tracer::MongoTracer};
use lazy_static::lazy_static;
use std::env;
use std::fs::canonicalize;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::{env::consts::ARCH, process::Command};
use tempfile::TempPath;

lazy_static! {
    static ref VALGRIND_BASE_ARGS: Vec<String> = {
        let mut args = vec![];
        args.extend(
            [
                // "-q",
                "--tool=callgrind",
                "--trace-children=yes",
                "--cache-sim=yes",
                "--I1=32768,8,64",
                "--D1=32768,8,64",
                "--LL=8388608,16,64",
                "--instr-atstart=no",
                "--collect-systime=nsec",
                "--compress-strings=no",
                "--combine-dumps=yes",
                "--dump-line=no",
            ]
            .iter()
            .map(|x| x.to_string()),
        );
        let children_skip_patterns = ["*esbuild"];
        args.push(format!(
            "--trace-children-skip={}",
            children_skip_patterns.join(",")
        ));
        args
    };
}

/// Creates the shell script on disk and returns the path to it.
fn create_run_script() -> anyhow::Result<TempPath> {
    // The command is wrapped in a shell script, which executes it in a
    // subprocess and then writes the exit code to a file. The process will
    // always exit with status code 0, unless valgrind fails.
    //
    // Args:
    // 1. The command to execute
    // 2. The path to the file where the exit code will be written
    const WRAPPER_SCRIPT: &str = r#"#!/usr/bin/env bash
bash -c "$1"
status=$?
echo -n "$status" > "$2"
"#;

    let rwx = std::fs::Permissions::from_mode(0o777);
    let mut script_file = tempfile::Builder::new()
        .suffix(".sh")
        .permissions(rwx)
        .tempfile()?;
    script_file.write_all(WRAPPER_SCRIPT.as_bytes())?;

    // Note: We have to convert the file to a path to be able to execute it.
    // Otherwise this will fail with 'File is busy' error.
    Ok(script_file.into_temp_path())
}

pub async fn measure(
    config: &Config,
    profile_folder: &Path,
    mongo_tracer: &Option<MongoTracer>,
) -> Result<()> {
    // Create the command
    let mut cmd = Command::new("setarch");
    cmd.arg(ARCH).arg("-R");
    // Configure the environment
    cmd.envs(get_base_injected_env(
        RunnerMode::Instrumentation,
        profile_folder,
    ))
    .env("PYTHONMALLOC", "malloc")
    .env(
        "PATH",
        format!(
            "{}:{}:{}",
            introspected_nodejs::setup()
                .map_err(|e| anyhow!("failed to setup NodeJS introspection. {e}"))?
                .to_string_lossy(),
            introspected_golang::setup()
                .map_err(|e| anyhow!("failed to setup Go introspection. {e}"))?
                .to_string_lossy(),
            env::var("PATH").unwrap_or_default(),
        ),
    );

    if let Some(cwd) = &config.working_directory {
        let abs_cwd = canonicalize(cwd)?;
        cmd.current_dir(abs_cwd);
    }
    // Configure valgrind
    let profile_path = profile_folder.join("%p.out");
    let log_path = profile_folder.join("valgrind.log");
    cmd.arg("valgrind")
        .args(VALGRIND_BASE_ARGS.iter())
        .args(
            get_objects_path_to_ignore()
                .iter()
                .map(|x| format!("--obj-skip={x}")),
        )
        .arg(format!("--callgrind-out-file={}", profile_path.to_str().unwrap()).as_str())
        .arg(format!("--log-file={}", log_path.to_str().unwrap()).as_str());

    // Set the command to execute:
    let script_path = create_run_script()?;
    let cmd_status_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    cmd.args([
        script_path.to_str().unwrap(),
        get_bench_command(config)?.as_str(),
        cmd_status_path.to_str().unwrap(),
    ]);

    // TODO: refactor and move this to the `Instruments` struct
    if let Some(mongo_tracer) = mongo_tracer {
        mongo_tracer.apply_run_command_transformations(&mut cmd)?;
    }

    debug!("cmd: {cmd:?}");
    let status = run_command_with_log_pipe(cmd)
        .await
        .map_err(|e| anyhow!("failed to execute the benchmark process. {e}"))?;
    debug!(
        "Valgrind exit code = {:?}, Valgrind signal = {:?}",
        status.code(),
        status.signal(),
    );

    // Check the valgrind exit code
    if !status.success() {
        let valgrind_log = profile_folder.join("valgrind.log");
        let valgrind_log = std::fs::read_to_string(&valgrind_log).unwrap_or_default();
        debug!("valgrind.log: {valgrind_log}");

        bail!("failed to execute valgrind");
    }

    // Check the exit code which was written to the file by the wrapper script.
    let cmd_status = {
        let content = std::fs::read_to_string(&cmd_status_path)?;
        content
            .parse::<u32>()
            .map_err(|e| anyhow!("unable to retrieve the program exit code. {e}"))?
    };
    debug!("Program exit code = {cmd_status}");
    if cmd_status != 0 {
        bail!("failed to execute the benchmark process, exit code: {cmd_status}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_run(to_execute: &str) -> (u32, u32) {
        let script_path = create_run_script().unwrap();
        let out_status = tempfile::NamedTempFile::new().unwrap().into_temp_path();

        let mut cmd = Command::new(script_path.to_str().unwrap());
        cmd.args([to_execute, out_status.to_str().unwrap()]);

        let script_status = cmd.status().unwrap().code().unwrap() as u32;
        let out_status = std::fs::read_to_string(out_status)
            .unwrap()
            .parse::<u32>()
            .unwrap();

        (script_status, out_status)
    }

    #[test]
    fn test_run_wrapper_script() {
        temp_env::with_var("TEST_ENV_VAR", "test_value".into(), || {
            assert_eq!(safe_run("echo $TEST_ENV_VAR"), (0, 0));
        });

        assert_eq!(safe_run("ls"), (0, 0));
        assert_eq!(safe_run("exit 0"), (0, 0));
        assert_eq!(safe_run("exit 1"), (0, 1));
        assert_eq!(safe_run("exit 255"), (0, 255));

        assert_eq!(safe_run("ls; exit 1"), (0, 1));
        assert_eq!(safe_run("exit 1; ls"), (0, 1));

        assert_eq!(safe_run("test 1 = 1 && exit 42"), (0, 42));
        assert_eq!(safe_run("test 1 = 2 && exit 42"), (0, 1));
        assert_eq!(safe_run("test 1 = 1 || exit 42"), (0, 0));
        assert_eq!(safe_run("test 1 = 2 || exit 42"), (0, 42));

        const MULTILINE_ECHO_SCRIPT: &str = "echo \"Working\"
echo \"with\"
echo \"multiple lines\"";

        const MULTILINE_ECHO_WITH_SEMICOLONS: &str = "echo \"Working\";
echo \"with\";
echo \"multiple lines\";";

        const ENV_VAR_VALIDATION_SCRIPT: &str = "export MY_ENV_VAR=\"Hello\"
output=$(echo \"$MY_ENV_VAR\")
if [ \"$output\" != \"Hello\" ]; then
  echo \"Assertion failed: Expected 'Hello' but got '$output'\"
  exit 1
fi";

        const ENV_VAR_VALIDATION_FAIL_SCRIPT: &str = "MY_ENV_VAR=\"Wrong\"
output=$(echo \"$MY_ENV_VAR\")
if [ \"$output\" != \"Hello\" ]; then
  echo \"Assertion failed: Expected 'Hello' but got '$output'\"
  exit 1
fi";

        const DIRECTORY_CHECK_SCRIPT: &str = "cd /tmp
# Check that the directory is actually changed
if [ $(basename $(pwd)) != \"tmp\" ]; then
  exit 1
fi";
        assert_eq!(safe_run(MULTILINE_ECHO_SCRIPT), (0, 0));
        assert_eq!(safe_run(MULTILINE_ECHO_WITH_SEMICOLONS), (0, 0));
        assert_eq!(safe_run(DIRECTORY_CHECK_SCRIPT), (0, 0));
        assert_eq!(safe_run(ENV_VAR_VALIDATION_SCRIPT), (0, 0));
        assert_eq!(safe_run(ENV_VAR_VALIDATION_FAIL_SCRIPT), (0, 1));
    }
}
