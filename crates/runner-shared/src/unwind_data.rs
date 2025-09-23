use core::{
    fmt::Debug,
    hash::{Hash, Hasher},
};
use serde::{Deserialize, Serialize};
use std::{hash::DefaultHasher, ops::Range};

/// Unwind data for a single module.
#[derive(Serialize, Deserialize)]
pub struct UnwindData {
    pub path: String,

    pub avma_range: Range<u64>,
    pub base_avma: u64,

    pub eh_frame_hdr: Vec<u8>,
    pub eh_frame_hdr_svma: Range<u64>,

    pub eh_frame: Vec<u8>,
    pub eh_frame_svma: Range<u64>,
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
            .field("avma_range", &format_args!("{:x?}", self.avma_range))
            .field("base_avma", &format_args!("{:x}", self.base_avma))
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
