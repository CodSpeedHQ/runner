use codspeed_runner::{clean_logger, cli};
use console::style;
use log::log_enabled;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let res = cli::run().await;
    if let Err(err) = res {
        for cause in err.chain() {
            if log_enabled!(log::Level::Error) {
                log::error!("{} {}", style("Error:").bold().red(), style(cause).red());
            } else {
                eprintln!("Error: {cause}");
            }
        }
        if log_enabled!(log::Level::Debug) {
            for e in err.chain().skip(1) {
                log::debug!("Caused by: {e}");
            }
        }
        clean_logger();
        std::process::exit(1);
    }
}
