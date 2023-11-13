mod app;
mod config;
mod prelude;
mod runner;
mod uploader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::init();
    let res = crate::app::run();
    if let Err(err) = res {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
