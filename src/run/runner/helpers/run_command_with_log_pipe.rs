use crate::local_logger::suspend_progress_bar;
use crate::prelude::*;
use std::io::{Read, Write};
use std::process::Command;
use std::process::ExitStatus;
use std::thread;

pub fn run_command_with_log_pipe(mut cmd: Command, target: &str) -> Result<ExitStatus> {
    fn log_tee(
        mut reader: impl Read,
        mut writer: impl Write,
        log_prefix: Option<&str>,
        target: &str,
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
                trace!(
                    target: target,
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
    let target_clone = target.to_string();
    thread::spawn(move || {
        log_tee(stdout, std::io::stdout(), None, &target_clone).unwrap();
    });

    let target_clone = target.to_string();
    thread::spawn(move || {
        log_tee(stderr, std::io::stderr(), Some("[stderr]"), &target_clone).unwrap();
    });
    process.wait().context("failed to wait for the process")
}
