use crate::prelude::*;
use std::path::Path;

use url::Url;

pub fn download_file(url: &Url, path: &Path) -> Result<()> {
    let response = reqwest::blocking::get(url.clone())
        .map_err(|e| anyhow!("Failed to download file: {}", e))?;
    if !response.status().is_success() {
        bail!("Failed to download file: {}", response.status());
    }
    let mut file = std::fs::File::create(path)
        .map_err(|e| anyhow!("Failed to create file: {}, {}", path.display(), e))?;
    let content = response
        .bytes()
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
