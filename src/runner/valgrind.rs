use crate::config::Config;
use crate::prelude::*;
use crate::runner::helpers::ignored_objects_path::get_objects_path_to_ignore;
use crate::runner::helpers::introspected_node::setup_introspected_node;
use lazy_static::lazy_static;
use std::env;
use std::fs::canonicalize;
use std::path::Path;
use std::{collections::HashMap, env::consts::ARCH, process::Command};

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

fn get_bench_command(config: &Config) -> String {
    let bench_command = &config.command;
    bench_command
        // Fixes a compatibility issue with cargo 1.66+ running directly under valgrind <3.20
        .replace("cargo codspeed", "cargo-codspeed")
}

pub fn measure(config: &Config, profile_folder: &Path) -> Result<()> {
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
    cmd.args(["sh", "-c", get_bench_command(config).as_str()]);

    debug!("cmd: {:?}", cmd);
    let status = cmd
        .status()
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
    fn test_get_bench_command_cargo() {
        let config = Config {
            command: "cargo codspeed bench".into(),
            ..Config::test()
        };
        assert_eq!(get_bench_command(&config), "cargo-codspeed bench");
    }

    #[test]
    fn test_get_bench_command_multiline() {
        let config = Config {
            command: r#"
cargo codspeed bench --features "foo bar"
pnpm vitest bench "my-app"
pytest tests/ --codspeed
"#
            .into(),
            ..Config::test()
        };
        assert_eq!(
            get_bench_command(&config),
            r#"
cargo-codspeed bench --features "foo bar"
pnpm vitest bench "my-app"
pytest tests/ --codspeed
"#
        );
    }
}
