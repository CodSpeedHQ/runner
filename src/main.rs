mod api_client;
mod app;
mod auth;
mod config;
mod local_logger;
mod logger;
mod prelude;
mod request_client;
mod run;

use console::style;
use local_logger::clean_logger;
use prelude::*;

use log::log_enabled;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";
pub const VALGRIND_CODSPEED_VERSION: &str = "3.21.0-0codspeed1";

#[tokio::main(flavor = "current_thread")]
async fn main() {
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
