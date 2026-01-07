/// Subset of perf events that CodSpeed supports.
#[derive(Debug, Clone, Copy)]
pub enum PerfEvent {
    CpuCycles,
    L1DCache,
    L2DCache,
    CacheMisses,
    Instructions,
}

impl PerfEvent {
    pub fn to_perf_string(&self) -> &'static str {
        match self {
            PerfEvent::CpuCycles => "cpu-cycles",
            PerfEvent::L1DCache => "l1d_cache",
            PerfEvent::L2DCache => "l2d_cache",
            PerfEvent::CacheMisses => "l2d_cache_refill",
            PerfEvent::Instructions => "instructions",
        }
    }

    pub fn all_events() -> Vec<PerfEvent> {
        vec![
            PerfEvent::CpuCycles,
            PerfEvent::L1DCache,
            PerfEvent::L2DCache,
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
