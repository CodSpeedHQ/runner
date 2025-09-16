use base64::{Engine, engine::general_purpose};

use crate::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ProfileArchive {
    pub hash: String,
    pub content: ProfileArchiveContent,
}

#[derive(Debug)]
pub enum ProfileArchiveContent {
    CompressedInMemory { data: Vec<u8> },
    UncompressedOnDisk { path: PathBuf },
}

impl ProfileArchive {
    pub fn new_compressed_in_memory(data: Vec<u8>) -> Self {
        let hash = general_purpose::STANDARD.encode(md5::compute(&data).0);
        ProfileArchive {
            hash,
            content: ProfileArchiveContent::CompressedInMemory { data },
        }
    }

    pub fn new_uncompressed_on_disk(path: PathBuf) -> Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        if !metadata.is_file() {
            return Err(anyhow!("The provided path is not a file"));
        }
        let mut file = std::fs::File::open(&path)?;
        let mut buffer = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut buffer)?;
        let hash = general_purpose::STANDARD.encode(md5::compute(&buffer).0);
        Ok(ProfileArchive {
            hash,
            content: ProfileArchiveContent::UncompressedOnDisk { path },
        })
    }
}

impl ProfileArchiveContent {
    pub async fn size(&self) -> Result<u64> {
        match &self {
            ProfileArchiveContent::CompressedInMemory { data } => Ok(data.len() as u64),
            ProfileArchiveContent::UncompressedOnDisk { path } => {
                let metadata = tokio::fs::metadata(path).await?;
                Ok(metadata.len())
            }
        }
    }

    pub fn encoding(&self) -> Option<String> {
        match self {
            ProfileArchiveContent::CompressedInMemory { .. } => Some("gzip".to_string()),
            _ => None,
        }
    }
}

impl Drop for ProfileArchiveContent {
    fn drop(&mut self) {
        if let ProfileArchiveContent::UncompressedOnDisk { path } = self {
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}
