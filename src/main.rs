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
use lazy_static::lazy_static;
use local_logger::clean_logger;
use prelude::*;
use semver::Version;

use log::log_enabled;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";

pub const VALGRIND_CODSPEED_VERSION: Version = Version::new(3, 24, 0);
pub const VALGRIND_CODSPEED_DEB_REVISION_SUFFIX: &str = "0codspeed1";
lazy_static! {
    pub static ref VALGRIND_CODSPEED_VERSION_STRING: String =
        format!("{VALGRIND_CODSPEED_VERSION}.codspeed");
    pub static ref VALGRIND_CODSPEED_DEB_VERSION: String =
        format!("{VALGRIND_CODSPEED_VERSION}-{VALGRIND_CODSPEED_DEB_REVISION_SUFFIX}");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let res = crate::app::run().await;
    if let Err(err) = res {
        for cause in err.chain() {
            if log_enabled!(log::Level::Error) {
                error!("{} {}", style("Error:").bold().red(), style(cause).red());
            } else {
                eprintln!("Error: {cause}");
            }
        }
        if log_enabled!(log::Level::Debug) {
            for e in err.chain().skip(1) {
                debug!("Caused by: {e}");
            }
        }
        clean_logger();
        std::process::exit(1);
    }
}
