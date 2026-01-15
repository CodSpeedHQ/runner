use core::{
    fmt::Debug,
    hash::{Hash, Hasher},
};
use serde::{Deserialize, Serialize};
use std::io::BufWriter;
use std::{hash::DefaultHasher, ops::Range};

pub const UNWIND_FILE_EXT: &str = "unwind_data";

pub type UnwindData = UnwindDataV2;

impl UnwindData {
    pub fn parse(reader: &[u8]) -> anyhow::Result<Self> {
        let compat: UnwindDataCompat = bincode::deserialize(reader)?;

        match compat {
            UnwindDataCompat::V1(v1) => Ok(v1.into()),
            UnwindDataCompat::V2(v2) => Ok(v2),
        }
    }

    pub fn save_to<P: AsRef<std::path::Path>>(&self, folder: P, pid: i32) -> anyhow::Result<()> {
        let unwind_data_path = folder.as_ref().join(format!(
            "{}_{:x}_{:x}_{}.{UNWIND_FILE_EXT}",
            pid,
            self.avma_range.start,
            self.avma_range.end,
            self.timestamp.unwrap_or_default()
        ));
        self.to_file(unwind_data_path)?;

        Ok(())
    }

    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        if let Ok(true) = std::fs::exists(path.as_ref()) {
            log::warn!(
                "{} already exists, file will be truncated",
                path.as_ref().display()
            );
            log::warn!("{} {:x?}", self.path, self.avma_range);
        }

        let compat = UnwindDataCompat::V2(self.clone());
        let file = std::fs::File::create(path.as_ref())?;
        const BUFFER_SIZE: usize = 256 * 1024 /* 256 KB */;

        let writer = BufWriter::with_capacity(BUFFER_SIZE, file);
        bincode::serialize_into(writer, &compat)?;

        Ok(())
    }
}

impl Debug for UnwindData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let eh_frame_hdr_hash = {
            let mut hasher = DefaultHasher::new();
            self.eh_frame_hdr.hash(&mut hasher);
            hasher.finish()
        };
        let eh_frame_hash = {
            let mut hasher = DefaultHasher::new();
            self.eh_frame.hash(&mut hasher);
            hasher.finish()
        };

        f.debug_struct("UnwindData")
            .field("path", &self.path)
            .field("timestamp", &self.timestamp)
            .field("avma_range", &format_args!("{:x?}", self.avma_range))
            .field("base_avma", &format_args!("{:x}", self.base_avma))
            .field("base_svma", &format_args!("{:x}", self.base_svma))
            .field(
                "eh_frame_hdr_svma",
                &format_args!("{:x?}", self.eh_frame_hdr_svma),
            )
            .field("eh_frame_hdr_hash", &format_args!("{eh_frame_hdr_hash:x}"))
            .field("eh_frame_hash", &format_args!("{eh_frame_hash:x}"))
            .field("eh_frame_svma", &format_args!("{:x?}", self.eh_frame_svma))
            .finish()
    }
}

/// A versioned enum for `UnwindData` to allow for future extensions while maintaining backward compatibility.
#[derive(Serialize, Deserialize)]
enum UnwindDataCompat {
    V1(UnwindDataV1),
    V2(UnwindDataV2),
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone)]
struct UnwindDataV1 {
    pub path: String,

    pub avma_range: Range<u64>,
    pub base_avma: u64,
    pub base_svma: u64,

    pub eh_frame_hdr: Vec<u8>,
    pub eh_frame_hdr_svma: Range<u64>,

    pub eh_frame: Vec<u8>,
    pub eh_frame_svma: Range<u64>,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone)]
pub struct UnwindDataV2 {
    pub path: String,

    /// The monotonic timestamp when the unwind data was captured.
    /// Is `None` if unwind data is valid for the whole program execution
    pub timestamp: Option<u64>,

    pub avma_range: Range<u64>,
    pub base_avma: u64,
    pub base_svma: u64,

    pub eh_frame_hdr: Vec<u8>,
    pub eh_frame_hdr_svma: Range<u64>,

    pub eh_frame: Vec<u8>,
    pub eh_frame_svma: Range<u64>,
}

impl From<UnwindDataV1> for UnwindDataV2 {
    fn from(v1: UnwindDataV1) -> Self {
        Self {
            path: v1.path,
            timestamp: None,
            avma_range: v1.avma_range,
            base_avma: v1.base_avma,
            base_svma: v1.base_svma,
            eh_frame_hdr: v1.eh_frame_hdr,
            eh_frame_hdr_svma: v1.eh_frame_hdr_svma,
            eh_frame: v1.eh_frame,
            eh_frame_svma: v1.eh_frame_svma,
        }
    }
}
