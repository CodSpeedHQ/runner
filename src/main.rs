mod app;
mod ci_provider;
mod config;
mod helpers;
mod instruments;
mod prelude;
mod request_client;
mod runner;
mod uploader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let res = crate::app::run().await;
    if let Err(err) = res {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
