mod api_client;
mod app;
mod auth;
mod config;
mod local_logger;
mod logger;
mod prelude;
mod request_client;
mod run;
mod setup;

use console::style;
use core_affinity::CoreId;
use lazy_static::lazy_static;
use local_logger::clean_logger;
use prelude::*;

use log::log_enabled;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";

const VALGRIND_CODSPEED_BASE_VERSION: &str = "3.24.0";
lazy_static! {
    pub static ref VALGRIND_CODSPEED_VERSION: String =
        format!("{VALGRIND_CODSPEED_BASE_VERSION}.codspeed");
    pub static ref VALGRIND_CODSPEED_DEB_VERSION: String =
        format!("{VALGRIND_CODSPEED_BASE_VERSION}-0codspeed1");
}

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .on_thread_start(|| {
            // Core 0 is also responsible for interrupts and other system tasks. But
            // this is fine, as we're not executing performance-critical code.
            if !core_affinity::set_for_current(CoreId { id: 0 }) {
                panic!("failed to pin worker thread to core 0");
            }
        })
        .enable_all()
        .build()?;

    // NOTE: The code above doesn't set the core affinity for the main thread. We should avoid doing so, as
    // the affinity is passed down to child threads/processes.

    let _guard = runtime.enter();
    runtime.block_on(real_main());
    Ok(())
}

async fn real_main() {
    let res = crate::app::run().await;
    if let Err(err) = res {
        for cause in err.chain() {
            if log_enabled!(log::Level::Error) {
                error!("{} {}", style("Error:").bold().red(), style(cause).red());
            } else {
                eprintln!("Error: {}", cause);
            }
        }
        if log_enabled!(log::Level::Debug) {
            for e in err.chain().skip(1) {
                debug!("Caused by: {}", e);
            }
        }
        clean_logger();
        std::process::exit(1);
    }
}
