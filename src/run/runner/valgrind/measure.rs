use crate::prelude::*;
use crate::run::runner::helpers::env::get_base_injected_env;
use crate::run::runner::helpers::get_bench_command::get_bench_command;
use crate::run::runner::helpers::run_command_with_log_pipe::run_command_with_log_pipe;
use crate::run::runner::valgrind::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::run::runner::valgrind::helpers::introspected_nodejs::setup_introspected_nodejs;
use crate::run::runner::RunnerMode;
use crate::run::{config::Config, instruments::mongo_tracer::MongoTracer};
use lazy_static::lazy_static;
use std::env;
use std::fs::canonicalize;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::{env::consts::ARCH, process::Command};

lazy_static! {
    static ref VALGRIND_BASE_ARGS: Vec<String> = {
        let mut args = vec![];
        args.extend(
            [
                "-q",
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

pub fn measure(
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
            "{}:{}",
            setup_introspected_nodejs()
                .map_err(|e| anyhow!("failed to setup NodeJS introspection. {}", e))?
                .to_str()
                .unwrap(),
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
                .map(|x| format!("--obj-skip={}", x)),
        )
        .arg(format!("--callgrind-out-file={}", profile_path.to_str().unwrap()).as_str())
        .arg(format!("--log-file={}", log_path.to_str().unwrap()).as_str());

    // The command is wrapped in a shell script, which executes it in a
    // subprocess and then writes the exit code to a file. The process will
    // always exit with status code 0, unless valgrind fails.
    let script_file = {
        const WRAPPER_SCRIPT: &str = r#"
            function safe_run()
            {
                (eval $1)
                local status=$?
                echo "$status" > "$2"
            }

            # Args:
            # 1. Cmd that should be executed
            # 2. File to write the status code to
            if [ "$#" -ne 2 ]; then
                echo "Usage: $0 <cmd> <file>"
                exit 1
            fi

            safe_run "$1" "$2"
            "#;

        let rwx = std::fs::Permissions::from_mode(0o777);
        let mut script_file = tempfile::Builder::new()
            .suffix(".sh")
            .permissions(rwx)
            .tempfile()?;
        script_file.write_all(WRAPPER_SCRIPT.as_bytes())?;
        script_file
    };

    // Set the command to execute:
    let mut cmd_status_file = tempfile::NamedTempFile::new()?;
    let run_cmd = format!(
        "{} \"{}\" {}",
        script_file.path().to_str().unwrap(),
        get_bench_command(config)?.as_str(),
        cmd_status_file.path().to_str().unwrap()
    );
    cmd.args(["sh", "-c", &run_cmd]);

    // TODO: refactor and move this to the `Instruments` struct
    if let Some(mongo_tracer) = mongo_tracer {
        mongo_tracer.apply_run_command_transformations(&mut cmd)?;
    }

    debug!("cmd: {:?}", cmd);
    let status = run_command_with_log_pipe(cmd)
        .map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?;
    let cmd_status = {
        let mut content = String::new();
        cmd_status_file.read_to_string(&mut content)?;

        content.trim().parse::<u32>()?
    };

    debug!(
        "Valgrind exit code = {:?}, Valgrind signal = {:?}, Program exit code = {}",
        status.code(),
        status.signal(),
        cmd_status
    );

    // Check the valgrind exit code
    if !status.success() {
        let valgrind_log = profile_folder.join("valgrind.log");
        let valgrind_log = std::fs::read_to_string(&valgrind_log).unwrap_or_default();
        debug!("valgrind.log: {}", valgrind_log);

        bail!("failed to execute valgrind");
    }

    // Check the exit code which was written to the file by the wrapper script.
    if cmd_status != 0 {
        bail!(
            "failed to execute the benchmark process, exit code: {}",
            cmd_status
        );
    }

    Ok(())
}
