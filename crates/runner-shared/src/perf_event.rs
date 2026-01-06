/// Subset of perf events that CodSpeed supports.
#[derive(Debug, Clone, Copy)]
pub enum PerfEvent {
    CpuCycles,
    CacheReferences,
    CacheMisses,
    Instructions,
}

impl PerfEvent {
    pub fn to_perf_string(&self) -> &'static str {
        match self {
            PerfEvent::CpuCycles => "cpu-cycles",
            PerfEvent::CacheReferences => "cache-references",
            PerfEvent::CacheMisses => "cache-misses",
            PerfEvent::Instructions => "instructions",
        }
    }

    pub fn from_perf_string(event: &str) -> Option<PerfEvent> {
        match event {
            "cpu-cycles" => Some(PerfEvent::CpuCycles),
            "cache-references" => Some(PerfEvent::CacheReferences),
            "cache-misses" => Some(PerfEvent::CacheMisses),
            "instructions" => Some(PerfEvent::Instructions),
            _ => None,
        }
    }

    pub fn all_events() -> Vec<PerfEvent> {
        vec![
            PerfEvent::CpuCycles,
            PerfEvent::CacheReferences,
            PerfEvent::CacheMisses,
            PerfEvent::Instructions,
        ]
    }
}

impl std::fmt::Display for PerfEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_perf_string())
    }
}
