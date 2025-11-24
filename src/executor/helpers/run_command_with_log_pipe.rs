use crate::executor::EXECUTOR_TARGET;
use crate::local_logger::suspend_progress_bar;
use crate::prelude::*;
use std::future::Future;
use std::io::{Read, Write};
use std::process::Command;
use std::process::ExitStatus;
use std::thread;

/// Run a command and log its output to stdout and stderr
///
/// # Arguments
/// - `cmd`: The command to run.
/// - `cb`: A callback function that takes the process ID and returns a result.
///
/// # Returns
///
/// The exit status of the command.
///
pub async fn run_command_with_log_pipe_and_callback<F, Fut>(
    mut cmd: Command,
    cb: F,
) -> Result<ExitStatus>
where
    F: FnOnce(u32) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    fn log_tee(
        mut reader: impl Read,
        mut writer: impl Write,
        log_prefix: Option<&str>,
    ) -> Result<()> {
        let prefix = log_prefix.unwrap_or("");
        let mut buffer = [0; 1024];
        let mut line_buffer = Vec::new();

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                // Flush any remaining data in the line buffer
                if !line_buffer.is_empty() {
                    suspend_progress_bar(|| {
                        writer.write_all(&line_buffer).unwrap();
                        trace!(
                            target: EXECUTOR_TARGET,
                            "{}{}",
                            prefix,
                            String::from_utf8_lossy(&line_buffer)
                        );
                    });
                }
                break;
            }

            // Add the chunk to our line buffer
            line_buffer.extend_from_slice(&buffer[..bytes_read]);

            // Check if we have any complete lines (ending with \n or \r)
            if let Some(last_newline_pos) =
                line_buffer.iter().rposition(|&b| b == b'\n' || b == b'\r')
            {
                // We have at least one complete line, flush up to and including the last newline
                let to_flush = &line_buffer[..=last_newline_pos];
                suspend_progress_bar(|| {
                    writer.write_all(to_flush).unwrap();
                    trace!(
                        target: EXECUTOR_TARGET,
                        "{}{}",
                        prefix,
                        String::from_utf8_lossy(to_flush)
                    );
                });

                // Keep the remainder in the buffer
                line_buffer = line_buffer[last_newline_pos + 1..].to_vec();
            }
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

    cb(process.id()).await?;

    process.wait().context("failed to wait for the process")
}

pub async fn run_command_with_log_pipe(cmd: Command) -> Result<ExitStatus> {
    run_command_with_log_pipe_and_callback(cmd, async |_| Ok(())).await
}
