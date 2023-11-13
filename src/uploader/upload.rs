use crate::{config::Config, prelude::*, runner::RunData};

pub fn upload(config: &Config, _run_data: &RunData) -> Result<()> {
    let client = reqwest::Client::new();
    let _ = client.post(config.upload_url.clone());
    Ok(())
}
