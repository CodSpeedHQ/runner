use crate::{prelude::*, request_client::REQUEST_CLIENT};
use std::path::Path;

use url::Url;

pub async fn download_file(url: &Url, path: &Path) -> Result<()> {
    debug!("Downloading file: {url}");
    let response = REQUEST_CLIENT
        .get(url.clone())
        .send()
        .await
        .map_err(|e| anyhow!("Failed to download file: {}", e))?;
    if !response.status().is_success() {
        bail!("Failed to download file: {}", response.status());
    }
    let mut file = std::fs::File::create(path)
        .map_err(|e| anyhow!("Failed to create file: {}, {}", path.display(), e))?;
    let content = response
        .bytes()
        .await
        .map_err(|e| anyhow!("Failed to read response: {}", e))?;
    std::io::copy(&mut content.as_ref(), &mut file).map_err(|e| {
        anyhow!(
            "Failed to write to file: {}, {}",
            path.display(),
            e.to_string()
        )
    })?;
    Ok(())
}
