mod app;
mod config;
mod prelude;
mod runner;
mod uploader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let res = crate::app::run().await;
    if let Err(err) = res {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
