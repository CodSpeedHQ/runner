use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::prelude::*;
use console::{Style, style};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use log::Log;
use simplelog::{CombinedLogger, SharedLogger};
use std::io::Write;

use crate::logger::{GroupEvent, JsonEvent, get_group_event, get_json_event};

pub const CODSPEED_U8_COLOR_CODE: u8 = 208; // #FF8700

lazy_static! {
    pub static ref SPINNER: Arc<Mutex<Option<ProgressBar>>> = Arc::new(Mutex::new(None));
    pub static ref IS_TTY: bool = std::io::IsTerminal::is_terminal(&std::io::stdout());
}

/// Hide the progress bar temporarily, execute `f`, then redraw the progress bar.
///
/// If the output is not a TTY, `f` will be executed without hiding the progress bar.
pub fn suspend_progress_bar<F: FnOnce() -> R, R>(f: F) -> R {
    // If the output is a TTY, and there is a spinner, suspend it
    if *IS_TTY {
        if let Ok(mut spinner) = SPINNER.lock() {
            if let Some(spinner) = spinner.as_mut() {
                return spinner.suspend(f);
            }
        }
    }

    // Otherwise, just run the function
    f()
}

pub struct LocalLogger {
    log_level: log::LevelFilter,
}

impl LocalLogger {
    pub fn new() -> Self {
        let log_level = env::var("CODSPEED_LOG")
            .ok()
            .and_then(|log_level| log_level.parse::<log::LevelFilter>().ok())
            .unwrap_or(log::LevelFilter::Info);

        LocalLogger { log_level }
    }
}

impl Log for LocalLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Some(group_event) = get_group_event(record) {
            match group_event {
                GroupEvent::Start(name) | GroupEvent::StartOpened(name) => {
                    eprintln!(
                        "\n{}",
                        style(format!("►►► {name} "))
                            .bold()
                            .color256(CODSPEED_U8_COLOR_CODE)
                    );

                    if *IS_TTY {
                        let spinner = ProgressBar::new_spinner();
                        spinner.set_style(
                            ProgressStyle::with_template(
                                format!(
                                    "  {{spinner:>.{CODSPEED_U8_COLOR_CODE}}} {{wide_msg:.{CODSPEED_U8_COLOR_CODE}.bold}}"
                                )
                                .as_str(),
                            )
                            .unwrap(),
                        );
                        spinner.set_message(format!("{name}..."));
                        spinner.enable_steady_tick(Duration::from_millis(100));
                        SPINNER.lock().unwrap().replace(spinner);
                    } else {
                        eprintln!("{name}...");
                    }
                }
                GroupEvent::End => {
                    if *IS_TTY {
                        let mut spinner = SPINNER.lock().unwrap();
                        if let Some(spinner) = spinner.as_mut() {
                            spinner.finish_and_clear();
                        }
                    }
                }
            }

            return;
        }

        if let Some(JsonEvent(json_string)) = get_json_event(record) {
            println!("{json_string}");
            return;
        }

        suspend_progress_bar(|| print_record(record));
    }

    fn flush(&self) {
        std::io::stdout().flush().unwrap();
    }
}

/// Print a log record to the console with the appropriate style
fn print_record(record: &log::Record) {
    let error_style = Style::new().red();
    let info_style = Style::new().white();
    let warn_style = Style::new().yellow();
    let debug_style = Style::new().blue().dim();
    let trace_style = Style::new().black().dim();

    match record.level() {
        log::Level::Error => eprintln!("{}", error_style.apply_to(record.args())),
        log::Level::Warn => eprintln!("{}", warn_style.apply_to(record.args())),
        log::Level::Info => eprintln!("{}", info_style.apply_to(record.args())),
        log::Level::Debug => eprintln!(
            "{}",
            debug_style.apply_to(format!("[DEBUG::{}] {}", record.target(), record.args())),
        ),
        log::Level::Trace => eprintln!(
            "{}",
            trace_style.apply_to(format!("[TRACE::{}] {}", record.target(), record.args()))
        ),
    }
}

impl SharedLogger for LocalLogger {
    fn level(&self) -> log::LevelFilter {
        self.log_level
    }

    fn config(&self) -> Option<&simplelog::Config> {
        None
    }

    fn as_log(self: Box<Self>) -> Box<dyn Log> {
        Box::new(*self)
    }
}

pub fn get_local_logger() -> Box<dyn SharedLogger> {
    Box::new(LocalLogger::new())
}

pub fn init_local_logger() -> Result<()> {
    let logger = get_local_logger();
    CombinedLogger::init(vec![logger])?;
    Ok(())
}

pub fn clean_logger() {
    let mut spinner = SPINNER.lock().unwrap();
    if let Some(spinner) = spinner.as_mut() {
        spinner.finish_and_clear();
    }
}
