use crate::local_logger::suspend_progress_bar;
use crate::prelude::*;
use crate::run::runner::EXECUTOR_TARGET;
use std::future::Future;
use std::io::{Read, Write};
use std::process::Command;
use std::process::ExitStatus;
use std::sync::{Arc, Mutex};
use std::thread;

struct CmdRunnerOptions<F> {
    on_process_spawned: Option<F>,
    capture_stdout: bool,
}

impl<F> Default for CmdRunnerOptions<F> {
    fn default() -> Self {
        Self {
            on_process_spawned: None,
            capture_stdout: false,
        }
    }
}

/// Run a command and log its output to stdout and stderr
///
/// # Arguments
/// - `cmd`: The command to run.
/// - `options`: Configuration options for the runner (e.g. capture output, run callback)
///
/// # Returns
/// A tuple containing:
/// - `ExitStatus`: The exit status of the executed command
/// - `Option<String>`: Captured stdout if `capture_stdout` was true, otherwise None
async fn run_command_with_log_pipe_and_options<F, Fut>(
    mut cmd: Command,
    options: CmdRunnerOptions<F>,
) -> Result<(ExitStatus, Option<String>)>
where
    F: FnOnce(u32) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    fn log_tee(
        mut reader: impl Read,
        mut writer: impl Write,
        log_prefix: Option<&str>,
        captured_output: Option<Arc<Mutex<Vec<u8>>>>,
    ) -> Result<()> {
        let prefix = log_prefix.unwrap_or("");
        let mut buffer = [0; 1024];
        let mut capture_guard = captured_output
            .as_ref()
            .map(|capture| capture.lock().unwrap());
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            if let Some(ref mut output) = capture_guard {
                output.extend_from_slice(&buffer[..bytes_read]);
            }

            suspend_progress_bar(|| {
                writer.write_all(&buffer[..bytes_read]).unwrap();
                trace!(
                    target: EXECUTOR_TARGET,
                    "{}{}",
                    prefix,
                    String::from_utf8_lossy(&buffer[..bytes_read])
                );
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

    let captured_stdout = if options.capture_stdout {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };
    let (stdout_handle, stderr_handle) = {
        let stdout_capture = captured_stdout.clone();
        let stdout_handle = thread::spawn(move || {
            log_tee(stdout, std::io::stdout(), None, stdout_capture).unwrap();
        });
        let stderr_handle = thread::spawn(move || {
            log_tee(stderr, std::io::stderr(), Some("[stderr]"), None).unwrap();
        });

        (stdout_handle, stderr_handle)
    };

    if let Some(cb) = options.on_process_spawned {
        cb(process.id()).await?;
    }

    let exit_status = process.wait().context("failed to wait for the process")?;
    let _ = (stdout_handle.join().unwrap(), stderr_handle.join().unwrap());

    let stdout_output = captured_stdout
        .map(|capture| String::from_utf8_lossy(&capture.lock().unwrap()).to_string());
    Ok((exit_status, stdout_output))
}

pub async fn run_command_with_log_pipe_and_callback<F, Fut>(
    cmd: Command,
    cb: F,
) -> Result<(ExitStatus, Option<String>)>
where
    F: FnOnce(u32) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    run_command_with_log_pipe_and_options(
        cmd,
        CmdRunnerOptions {
            on_process_spawned: Some(cb),
            capture_stdout: false,
        },
    )
    .await
}

pub async fn run_command_with_log_pipe(cmd: Command) -> Result<ExitStatus> {
    let (exit_status, _) = run_command_with_log_pipe_and_options(
        cmd,
        CmdRunnerOptions::<fn(u32) -> futures::future::Ready<anyhow::Result<()>>> {
            on_process_spawned: None,
            capture_stdout: false,
        },
    )
    .await?;
    Ok(exit_status)
}

pub async fn run_command_with_log_pipe_capture_stdout(
    cmd: Command,
) -> Result<(ExitStatus, String)> {
    let (exit_status, stdout) = run_command_with_log_pipe_and_options(
        cmd,
        CmdRunnerOptions::<fn(u32) -> futures::future::Ready<anyhow::Result<()>>> {
            on_process_spawned: None,
            capture_stdout: true,
        },
    )
    .await?;
    Ok((exit_status, stdout.unwrap_or_default()))
}
