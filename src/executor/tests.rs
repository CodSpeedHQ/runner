use crate::executor::Executor;
use crate::executor::interfaces::RunData;
use crate::executor::valgrind::executor::ValgrindExecutor;
use crate::executor::wall_time::executor::WallTimeExecutor;
use crate::run::check_system::SystemInfo;
use crate::run::config::Config;
use crate::runner_mode::RunnerMode;
use rstest_reuse::{self, *};
use shell_quote::{Bash, QuoteRefExt};
use tempfile::TempDir;
use tokio::sync::{OnceCell, Semaphore, SemaphorePermit};

const TESTS: [&str; 6] = [
    // Simple echo command
    "echo 'Hello, World!'",
    // Multi-line commands without semicolons
    "echo \"Working\"
echo \"with\"
echo \"multiple lines\"",
    // Multi-line commands with semicolons
    "echo \"Working\";
echo \"with\";
echo \"multiple lines\";",
    // Directory change and validation
    "cd /tmp
# Check that the directory is actually changed
if [ $(basename $(pwd)) != \"tmp\" ]; then
  exit 1
fi",
    // Quote escaping test
    "#!/bin/bash
VALUE=\"He said \\\"Hello 'world'\\\" & echo \\$HOME\"
if [ \"$VALUE\" = \"He said \\\"Hello 'world'\\\" & echo \\$HOME\" ]; then
  echo \"Quote test passed\"
else
  echo \"ERROR: Quote handling failed\"
  exit 1
fi",
    // Command substitution test
    "#!/bin/bash
RESULT=$(echo \"test 'nested' \\\"quotes\\\" here\")
COUNT=$(echo \"$RESULT\" | wc -w)
if [ \"$COUNT\" -eq \"4\" ]; then
  echo \"Command substitution test passed\"
else
  echo \"ERROR: Expected 4 words, got $COUNT\"
  exit 1
fi",
];

fn env_var_validation_script(env: &str, expected: &str) -> String {
    let expected: String = expected.quoted(Bash);
    format!(
        r#"
if [ "${env}" != {expected} ]; then
  echo "FAIL: Environment variable not set correctly"
  echo "Got: '${env}'"
  exit 1
fi
"#
    )
}

const ENV_TESTS: [(&str, &str); 8] = [
    // Mixed quotes, backticks, and shell metacharacters
    (
        "quotes_and_escapes",
        r#""'He said "Hello 'world' `date`" & echo "done" with \\n\\t\\"#,
    ),
    // Multiline content with tabs and trailing whitespace
    (
        "multiline_and_whitespace",
        "Line 1\nLine 2\tTabbed\n   \t  ",
    ),
    // Shell metacharacters: pipes, redirects, operators
    (
        "shell_metacharacters",
        r#"*.txt | grep "test" && echo "found" || echo "error" ; ls > /tmp/out"#,
    ),
    // Variable expansion and command substitution
    (
        "variables_and_commands",
        r#"$HOME ${PATH} $((1+1)) $(echo "embedded") VAR="value with spaces""#,
    ),
    // Unicode characters and ANSI escape sequences
    (
        "unicode_and_special",
        "🚀 café naïve\u{200b}hidden\x1b[31mRed\x1b[0m\x01\x02",
    ),
    // Complex mix of quoting styles with shell operators
    (
        "complex_mixed",
        r#"start'single'middle"double"end $VAR | cmd && echo "done" || fail"#,
    ),
    // Empty string edge case
    ("empty", ""),
    // Whitespace-only content
    ("space_only", "   "),
];

#[template]
#[rstest::rstest]
#[case(TESTS[0])]
#[case(TESTS[1])]
#[case(TESTS[2])]
#[case(TESTS[3])]
#[case(TESTS[4])]
#[case(TESTS[5])]
fn test_cases(#[case] cmd: &str) {}

#[template]
#[rstest::rstest]
#[case(ENV_TESTS[0])]
#[case(ENV_TESTS[1])]
#[case(ENV_TESTS[2])]
#[case(ENV_TESTS[3])]
#[case(ENV_TESTS[4])]
#[case(ENV_TESTS[5])]
#[case(ENV_TESTS[6])]
#[case(ENV_TESTS[7])]
fn env_test_cases(#[case] env_case: (&str, &str)) {}

async fn create_test_setup() -> (SystemInfo, RunData, TempDir) {
    let system_info = SystemInfo::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let run_data = RunData {
        profile_folder: temp_dir.path().to_path_buf(),
    };
    (system_info, run_data, temp_dir)
}

mod valgrind {
    use super::*;

    async fn get_valgrind_executor() -> &'static ValgrindExecutor {
        static VALGRIND_EXECUTOR: OnceCell<ValgrindExecutor> = OnceCell::const_new();

        VALGRIND_EXECUTOR
            .get_or_init(|| async {
                let executor = ValgrindExecutor;
                let system_info = SystemInfo::new().unwrap();
                executor.setup(&system_info, None).await.unwrap();
                executor
            })
            .await
    }

    #[cfg(test)]
    fn valgrind_config(command: &str) -> Config {
        Config {
            mode: RunnerMode::Simulation,
            command: command.to_string(),
            ..Config::test()
        }
    }

    #[apply(test_cases)]
    #[tokio::test]
    async fn test_valgrind_executor(#[case] cmd: &str) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let executor = get_valgrind_executor().await;

        let config = valgrind_config(cmd);
        executor
            .run(&config, &system_info, &run_data, &None)
            .await
            .unwrap();
    }

    #[apply(env_test_cases)]
    #[tokio::test]
    async fn test_valgrind_executor_with_env(#[case] env_case: (&str, &str)) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let executor = get_valgrind_executor().await;

        let (env_var, env_value) = env_case;
        temp_env::async_with_vars(&[(env_var, Some(env_value))], async {
            let cmd = env_var_validation_script(env_var, env_value);
            let config = valgrind_config(&cmd);
            executor
                .run(&config, &system_info, &run_data, &None)
                .await
                .unwrap();
        })
        .await;
    }
}

mod walltime {
    use super::*;

    async fn get_walltime_executor() -> (SemaphorePermit<'static>, WallTimeExecutor) {
        static WALLTIME_INIT: OnceCell<()> = OnceCell::const_new();
        static WALLTIME_SEMAPHORE: OnceCell<Semaphore> = OnceCell::const_new();

        WALLTIME_INIT
            .get_or_init(|| async {
                let executor = WallTimeExecutor::new();
                let system_info = SystemInfo::new().unwrap();
                executor.setup(&system_info, None).await.unwrap();
            })
            .await;

        // We can't execute multiple walltime executors in parallel because perf isn't thread-safe (yet). We have to
        // use a semaphore to limit concurrent access.
        let semaphore = WALLTIME_SEMAPHORE
            .get_or_init(|| async { Semaphore::new(1) })
            .await;
        let permit = semaphore.acquire().await.unwrap();

        (permit, WallTimeExecutor::new())
    }

    fn walltime_config(command: &str, enable_perf: bool) -> Config {
        Config {
            mode: RunnerMode::Walltime,
            command: command.to_string(),
            enable_perf,
            ..Config::test()
        }
    }

    #[apply(test_cases)]
    #[rstest::rstest]
    #[tokio::test]
    async fn test_walltime_executor(#[case] cmd: &str, #[values(false, true)] enable_perf: bool) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        let config = walltime_config(cmd, enable_perf);
        executor
            .run(&config, &system_info, &run_data, &None)
            .await
            .unwrap();
    }

    #[apply(env_test_cases)]
    #[rstest::rstest]
    #[tokio::test]
    async fn test_walltime_executor_with_env(
        #[case] env_case: (&str, &str),
        #[values(false, true)] enable_perf: bool,
    ) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        let (env_var, env_value) = env_case;
        temp_env::async_with_vars(&[(env_var, Some(env_value))], async {
            let cmd = env_var_validation_script(env_var, env_value);
            let config = walltime_config(&cmd, enable_perf);
            executor
                .run(&config, &system_info, &run_data, &None)
                .await
                .unwrap();
        })
        .await;
    }

    // Ensure that the working directory is used correctly
    #[rstest::rstest]
    #[tokio::test]
    async fn test_walltime_executor_in_working_dir(#[values(false, true)] enable_perf: bool) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        let cmd = r#"
if [ "$(basename "$(pwd)")" != "within_sub_directory" ]; then
    echo "FAIL: Working directory is not 'within_sub_directory'"
    exit 1
fi
"#;

        let mut config = walltime_config(cmd, enable_perf);

        let dir = TempDir::new().unwrap();
        config.working_directory = Some(
            dir.path()
                .join("within_sub_directory")
                .to_string_lossy()
                .to_string(),
        );
        std::fs::create_dir_all(config.working_directory.as_ref().unwrap()).unwrap();

        executor
            .run(&config, &system_info, &run_data, &None)
            .await
            .unwrap();
    }

    // Ensure that commands that fail actually fail
    #[rstest::rstest]
    #[tokio::test]
    async fn test_walltime_executor_fails(#[values(false, true)] enable_perf: bool) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        let config = walltime_config("exit 1", enable_perf);
        let result = executor.run(&config, &system_info, &run_data, &None).await;
        assert!(result.is_err(), "Command should fail");
    }
}
