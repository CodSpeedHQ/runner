mod app;
mod ci_provider;
mod config;
mod helpers;
mod instruments;
mod prelude;
mod request_client;
mod runner;
mod uploader;

use log::log_enabled;
use prelude::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";
pub const VALGRIND_CODSPEED_VERSION: &str = "3.21.0-0codspeed1";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let res = crate::app::run().await;
    if let Err(err) = res {
        if log_enabled!(log::Level::Error) {
            error!("Error {}", err);
        } else {
            eprintln!("Error {}", err);
        }
        std::process::exit(1);
    }
}
