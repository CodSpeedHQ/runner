use crate::run::check_system::SystemInfo;
use crate::run::config::Config;
use crate::run::runner::executor::Executor;
use crate::run::runner::interfaces::RunData;
use crate::run::runner::valgrind::executor::ValgrindExecutor;
use crate::run::{RunnerMode, runner::wall_time::executor::WallTimeExecutor};
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
const ENV_VAR_VALIDATION_SCRIPT: &str = "
output=$(echo \"$MY_ENV_VAR\")
if [ \"$output\" != \"Hello\" ]; then
  echo \"Assertion failed: Expected 'Hello' but got '$output'\"
  exit 1
fi";

const TESTS: [&str; 5] = [
    SIMPLE_ECHO_SCRIPT,
    MULTILINE_ECHO_SCRIPT,
    MULTILINE_ECHO_WITH_SEMICOLONS,
    DIRECTORY_CHECK_SCRIPT,
    ENV_VAR_VALIDATION_SCRIPT,
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
    #[case("MY_ENV_VAR", "Hello", ENV_VAR_VALIDATION_SCRIPT)]
    #[tokio::test]
    async fn test_valgrind_executor_with_env(
        #[case] env_var: &str,
        #[case] env_value: &str,
        #[case] cmd: &str,
    ) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let executor = get_valgrind_executor().await;

        temp_env::async_with_vars(&[(env_var, Some(env_value))], async {
            let config = valgrind_config(cmd);
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
    #[case("MY_ENV_VAR", "Hello", ENV_VAR_VALIDATION_SCRIPT, false)]
    #[case("MY_ENV_VAR", "Hello", ENV_VAR_VALIDATION_SCRIPT, true)]
    #[tokio::test]
    async fn test_walltime_executor_with_env(
        #[case] env_var: &str,
        #[case] env_value: &str,
        #[case] cmd: &str,
        #[case] enable_perf: bool,
    ) {
        let (system_info, run_data, _temp_dir) = create_test_setup().await;
        let (_permit, executor) = get_walltime_executor().await;

        temp_env::async_with_vars(&[(env_var, Some(env_value))], async {
            let config = walltime_config(cmd, enable_perf);
            executor
                .run(&config, &system_info, &run_data, &None)
                .await
                .unwrap();
        })
        .await;
    }
}
