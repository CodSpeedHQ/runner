use crate::local_logger::suspend_progress_bar;
use crate::prelude::*;
use crate::run::runner::valgrind::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::run::runner::valgrind::helpers::introspected_node::setup_introspected_node;
use crate::run::{config::Config, instruments::mongo_tracer::MongoTracer};
use lazy_static::lazy_static;
use std::fs::canonicalize;
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitStatus;
use std::{collections::HashMap, env::consts::ARCH, process::Command};
use std::{env, thread};

lazy_static! {
    static ref BASE_INJECTED_ENV: HashMap<&'static str, String> = {
        HashMap::from([
            ("PYTHONMALLOC", "malloc".into()),
            ("PYTHONHASHSEED", "0".into()),
            ("ARCH", ARCH.into()),
            ("CODSPEED_ENV", "runner".into()),
        ])
    };
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

fn get_bench_command(config: &Config) -> Result<String> {
    let bench_command = &config.command.trim();

    if bench_command.is_empty() {
        bail!("The bench command is empty");
    }

    Ok(bench_command
        // Fixes a compatibility issue with cargo 1.66+ running directly under valgrind <3.20
        .replace("cargo codspeed", "cargo-codspeed"))
}

pub const VALGRIND_EXECUTION_TARGET: &str = "valgrind::execution";

fn run_command_with_log_pipe(mut cmd: Command) -> Result<ExitStatus> {
    fn log_tee(
        mut reader: impl Read,
        mut writer: impl Write,
        log_prefix: Option<&str>,
    ) -> Result<()> {
        let prefix = log_prefix.unwrap_or("");
        let mut buffer = [0; 1024];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            suspend_progress_bar(|| {
                writer.write_all(&buffer[..bytes_read]).unwrap();
                trace!(target: VALGRIND_EXECUTION_TARGET, "{}{}", prefix, String::from_utf8_lossy(&buffer[..bytes_read]));
            });
        }
        Ok(())
    }

    let mut process = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn the process")?;
    let stdout = process.stdout.take().expect("unable to get stdout");
    let stderr = process.stderr.take().expect("unable to get stderr");
    thread::spawn(move || {
        log_tee(stdout, std::io::stdout(), None).unwrap();
    });
    thread::spawn(move || {
        log_tee(stderr, std::io::stderr(), Some("[stderr]")).unwrap();
    });
    process.wait().context("failed to wait for the process")
}

pub fn measure(
    config: &Config,
    profile_folder: &Path,
    mongo_tracer: &Option<MongoTracer>,
) -> Result<()> {
    debug!("profile dir: {}", profile_folder.display());

    // Create the command
    let mut cmd = Command::new("setarch");
    cmd.arg(ARCH).arg("-R");
    // Configure the environment
    cmd.envs(BASE_INJECTED_ENV.iter()).env(
        "PATH",
        format!(
            "{}:{}",
            setup_introspected_node()
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

    // Set the command to execute
    cmd.args(["sh", "-c", get_bench_command(config)?.as_str()]);

    // TODO: refactor and move this to the `Instruments` struct
    if let Some(mongo_tracer) = mongo_tracer {
        mongo_tracer.apply_run_command_transformations(&mut cmd)?;
    }

    debug!("cmd: {:?}", cmd);
    let status = run_command_with_log_pipe(cmd)
        .map_err(|e| anyhow!("failed to execute the benchmark process. {}", e))?;
    if !status.success() {
        bail!("failed to execute the benchmark process");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bench_command_empty() {
        let config = Config::test();
        assert!(get_bench_command(&config).is_err());
        assert_eq!(
            get_bench_command(&config).unwrap_err().to_string(),
            "The bench command is empty"
        );
    }

    #[test]
    fn test_get_bench_command_cargo() {
        let config = Config {
            command: "cargo codspeed bench".into(),
            ..Config::test()
        };
        assert_eq!(get_bench_command(&config).unwrap(), "cargo-codspeed bench");
    }

    #[test]
    fn test_get_bench_command_multiline() {
        let config = Config {
            // TODO: use indoc! macro
            command: r#"
cargo codspeed bench --features "foo bar"
pnpm vitest bench "my-app"
pytest tests/ --codspeed
"#
            .into(),
            ..Config::test()
        };
        assert_eq!(
            get_bench_command(&config).unwrap(),
            r#"cargo-codspeed bench --features "foo bar"
pnpm vitest bench "my-app"
pytest tests/ --codspeed"#
        );
    }
}
