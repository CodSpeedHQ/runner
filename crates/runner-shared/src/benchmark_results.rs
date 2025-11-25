use log::debug;
pub use prost::Message;

// Include the generated protobuf code
include!(concat!(
    env!("OUT_DIR"),
    "/codspeed.benchmark_results.v1.rs"
));

pub trait BenchmarkResultExt: Message
where
    Self: Sized,
{
    fn name() -> &'static str;

    fn save_file_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        filename: &str,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(folder.as_ref())?;
        std::fs::write(folder.as_ref().join(filename), self.encode_to_vec())?;

        debug!("Saved {} result to {:?}", Self::name(), folder.as_ref());
        Ok(())
    }

    fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{}.binpb", Self::name()))
    }

    fn save_with_pid_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        pid: u32,
    ) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{pid}.{}.binpb", Self::name()))
    }
}

impl BenchmarkResultExt for HeaptrackResult {
    fn name() -> &'static str {
        "heaptrack"
    }
}

impl BenchmarkResultExt for MarkerResult {
    fn name() -> &'static str {
        "markers"
    }
}

impl MarkerResult {
    pub fn new(uri_by_ts: &[(u64, String)], markers: &[crate::fifo::MarkerType]) -> Self {
        Self {
            uri_by_ts: uri_by_ts
                .iter()
                .map(|(ts, uri)| BenchmarkUri {
                    timestamp: *ts,
                    uri: uri.clone(),
                })
                .collect(),
            markers: markers.iter().map(|m| Marker::from(*m)).collect(),
        }
    }
}

impl From<crate::fifo::MarkerType> for Marker {
    fn from(value: crate::fifo::MarkerType) -> Self {
        match value {
            crate::fifo::MarkerType::SampleStart(ts) => Marker {
                r#type: MarkerType::SampleStart as i32,
                timestamp: ts,
            },
            crate::fifo::MarkerType::SampleEnd(ts) => Marker {
                r#type: MarkerType::SampleEnd as i32,
                timestamp: ts,
            },
            crate::fifo::MarkerType::BenchmarkStart(ts) => Marker {
                r#type: MarkerType::BenchmarkStart as i32,
                timestamp: ts,
            },
            crate::fifo::MarkerType::BenchmarkEnd(ts) => Marker {
                r#type: MarkerType::BenchmarkEnd as i32,
                timestamp: ts,
            },
        }
    }
}

impl From<Marker> for crate::fifo::MarkerType {
    fn from(value: Marker) -> Self {
        match MarkerType::try_from(value.r#type).unwrap() {
            MarkerType::SampleStart => crate::fifo::MarkerType::SampleStart(value.timestamp),
            MarkerType::SampleEnd => crate::fifo::MarkerType::SampleEnd(value.timestamp),
            MarkerType::BenchmarkStart => crate::fifo::MarkerType::BenchmarkStart(value.timestamp),
            MarkerType::BenchmarkEnd => crate::fifo::MarkerType::BenchmarkEnd(value.timestamp),
        }
    }
}
