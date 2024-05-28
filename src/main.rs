mod app;
mod auth;
mod config;
mod logger;
mod prelude;
mod request_client;
mod run;

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
                error!("Error {}", cause);
            } else {
                eprintln!("Error {}", cause);
            }
        }
        if log_enabled!(log::Level::Debug) {
            for e in err.chain().skip(1) {
                debug!("Caused by: {}", e);
            }
        }
        std::process::exit(1);
    }
}
