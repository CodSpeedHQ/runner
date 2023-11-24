use env_logger::Env;

mod app;
mod config;
mod prelude;
mod request_client;
mod runner;
mod uploader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let res = crate::app::run().await;
    if let Err(err) = res {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
