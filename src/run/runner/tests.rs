use crate::run::check_system::SystemInfo;
use crate::run::config::Config;
use crate::run::runner::executor::Executor;
use crate::run::runner::interfaces::RunData;
use crate::run::runner::valgrind::executor::ValgrindExecutor;
use crate::run::{RunnerMode, runner::wall_time::executor::WallTimeExecutor};
use shell_quote::{Bash, QuoteRefExt};
use tempfile::TempDir;
use tokio::sync::{OnceCell, Semaphore, SemaphorePermit};

const SIMPLE_ECHO_SCRIPT: &str = "echo 'Hello, World!'";
const MULTILINE_ECHO_SCRIPT: &str = "echo \"Working\"
echo \"with\"
echo \"multiple lines\"";
const MULTILINE_ECHO_WITH_SEMICOLONS: &str = "echo \"Working\";
echo \"with\";
echo \"multiple lines\";";
const DIRECTORY_CHECK_SCRIPT: &str = "cd /tmp
# Check that the directory is actually changed
if [ $(basename $(pwd)) != \"tmp\" ]; then
  exit 1
fi";
const QUOTE_ESCAPE_SCRIPT: &str = "#!/bin/bash
VALUE=\"He said \\\"Hello 'world'\\\" & echo \\$HOME\"
if [ \"$VALUE\" = \"He said \\\"Hello 'world'\\\" & echo \\$HOME\" ]; then
  echo \"Quote test passed\"
else
  echo \"ERROR: Quote handling failed\"
  exit 1
fi";
const COMMAND_SUBSTITUTION_SCRIPT: &str = "#!/bin/bash
RESULT=$(echo \"test 'nested' \\\"quotes\\\" here\")
COUNT=$(echo \"$RESULT\" | wc -w)
if [ \"$COUNT\" -eq \"4\" ]; then
  echo \"Command substitution test passed\"
else
  echo \"ERROR: Expected 4 words, got $COUNT\"
  exit 1
fi";

const TESTS: [&str; 6] = [
    SIMPLE_ECHO_SCRIPT,
    MULTILINE_ECHO_SCRIPT,
    MULTILINE_ECHO_WITH_SEMICOLONS,
    DIRECTORY_CHECK_SCRIPT,
    QUOTE_ESCAPE_SCRIPT,
    COMMAND_SUBSTITUTION_SCRIPT,
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

const QUOTES_AND_ESCAPES: &str = r#""'He said "Hello 'world' `date`" & echo "done" with \\n\\t\\"#;
const MULTILINE_AND_WHITESPACE: &str = "Line 1\nLine 2\tTabbed\n   \t  ";
const SHELL_METACHARACTERS: &str =
    r#"*.txt | grep "test" && echo "found" || echo "error" ; ls > /tmp/out"#;
const VARIABLES_AND_COMMANDS: &str =
    r#"$HOME ${PATH} $((1+1)) $(echo "embedded") VAR="value with spaces""#;
const UNICODE_AND_SPECIAL: &str = "🚀 café naïve\u{200b}hidden\x1b[31mRed\x1b[0m\x01\x02";
const COMPLEX_MIXED: &str = r#"start'single'middle"double"end $VAR | cmd && echo "done" || fail"#;
const EMPTY: &str = "";
const SPACE_ONLY: &str = "   ";

const ENV_TESTS: [(&str, &str); 8] = [
    ("quotes_and_escapes", QUOTES_AND_ESCAPES),
    ("multiline_and_whitespace", MULTILINE_AND_WHITESPACE),
    ("shell_metacharacters", SHELL_METACHARACTERS),
    ("variables_and_commands", VARIABLES_AND_COMMANDS),
    ("unicode_and_special", UNICODE_AND_SPECIAL),
    ("complex_mixed", COMPLEX_MIXED),
    ("empty", EMPTY),
    ("space_only", SPACE_ONLY),
];

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
                executor.setup(&system_info).await.unwrap();
                executor
            })
            .await
    }

    fn valgrind_config(command: &str) -> Config {
        Config {
            mode: RunnerMode::Instrumentation,
            command: command.to_string(),
            ..Config::test()
        }
    }

    #[rstest::rstest]
    #[case(TESTS[0])]
    #[case(TESTS[1])]
    #[case(TESTS[2])]
    #[case(TESTS[3])]
    #[case(TESTS[4])]
    #[case(TESTS[5])]
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

    #[rstest::rstest]
    #[case(ENV_TESTS[0])]
    #[case(ENV_TESTS[1])]
    #[case(ENV_TESTS[2])]
    #[case(ENV_TESTS[3])]
    #[case(ENV_TESTS[4])]
    #[case(ENV_TESTS[5])]
    #[case(ENV_TESTS[6])]
    #[case(ENV_TESTS[7])]
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
                executor.setup(&system_info).await.unwrap();
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

    #[rstest::rstest]
    #[case(TESTS[0], false)]
    #[case(TESTS[0], true)]
    #[case(TESTS[1], false)]
    #[case(TESTS[1], true)]
    #[case(TESTS[2], false)]
    #[case(TESTS[2], true)]
    #[case(TESTS[3], false)]
    #[case(TESTS[3], true)]
    #[case(TESTS[4], false)]
    #[case(TESTS[4], true)]
    #[case(TESTS[5], false)]
    #[case(TESTS[5], true)]
    #[tokio::test]
    async fn test_walltime_executor(#[case] cmd: &str, #[case] enable_perf: bool) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        let config = walltime_config(cmd, enable_perf);
        executor
            .run(&config, &system_info, &run_data, &None)
            .await
            .unwrap();
    }

    #[rstest::rstest]
    #[case(ENV_TESTS[0], false)]
    #[case(ENV_TESTS[0], true)]
    #[case(ENV_TESTS[1], false)]
    #[case(ENV_TESTS[1], true)]
    #[case(ENV_TESTS[2], false)]
    #[case(ENV_TESTS[2], true)]
    #[case(ENV_TESTS[3], false)]
    #[case(ENV_TESTS[3], true)]
    #[case(ENV_TESTS[4], false)]
    #[case(ENV_TESTS[4], true)]
    #[case(ENV_TESTS[5], false)]
    #[case(ENV_TESTS[5], true)]
    #[case(ENV_TESTS[6], false)]
    #[case(ENV_TESTS[6], true)]
    #[case(ENV_TESTS[7], false)]
    #[case(ENV_TESTS[7], true)]
    #[tokio::test]
    async fn test_walltime_executor_with_env(
        #[case] env_case: (&str, &str),
        #[case] enable_perf: bool,
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
}
