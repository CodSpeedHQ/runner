use super::setup::run_with_sudo;
use crate::prelude::*;

pub struct CpuIsolation;

impl CpuIsolation {
    pub fn new() -> Self {
        Self {}
    }

    fn set_allowed_cpus(scope: &str, allowed_cpus: &[u32]) -> anyhow::Result<()> {
        let cpus = allowed_cpus
            .iter()
            .map(|cpu| cpu.to_string())
            .collect::<Vec<_>>()
            .join(",");

        run_with_sudo(&[
            "systemctl",
            "set-property",
            scope,
            &format!("AllowedCPUs={}", cpus),
        ])?;

        Ok(())
    }

    pub fn isolate(&self) -> anyhow::Result<()> {
        let (instrument_cpu, bench_cpu, system_cpus) = Self::cpu_bindings();
        Self::set_allowed_cpus("system.slice", &system_cpus)?;
        Self::set_allowed_cpus("init.scope", &system_cpus)?;

        let user_cpus = vec![instrument_cpu, bench_cpu];
        Self::set_allowed_cpus("user.slice", &user_cpus)?;

        Ok(())
    }

    pub fn reset_isolation(&self) -> anyhow::Result<()> {
        let root_cpus = Self::root_cpus().unwrap_or_default();
        Self::set_allowed_cpus("system.slice", &root_cpus)?;
        Self::set_allowed_cpus("user.slice", &root_cpus)?;
        Self::set_allowed_cpus("init.scope", &root_cpus)?;

        Ok(())
    }

    /// Find the CPU cores to use for the isolated tasks:
    /// 1. System processes
    /// 2. Perf process / other instruments
    /// 3. Benchmarks
    ///
    /// If isolated CPUs are available, they will be used for the perf and benchmark processes. The
    /// remaining online CPUs will be used for system processes.
    pub fn cpu_bindings() -> (u32, u32, Vec<u32>) {
        // Use system isolated cpus (done via boot parameter) which we can use for the benchmark
        // and perf process. If not isolated CPUs exist, we will use the default ones.
        let isolated = Self::isolated_cpus().unwrap_or_default();
        debug!("Isolated CPUs: {isolated:?}");

        let root = Self::root_cpus().unwrap_or_default();
        debug!("Root CPUs: {root:?}");

        assert!(
            root.len() + isolated.len() >= 3,
            "At least 3 CPUs are required"
        );

        let (bench_cpu, system_cpus) = if isolated.is_empty() {
            (root[0], root[1..].to_vec())
        } else {
            (isolated[0], root)
        };

        // WARN: The instrument cpu must also be a system CPU, otherwise perf will not work.
        let instrument_cpu = system_cpus[0];

        (instrument_cpu, bench_cpu, system_cpus)
    }

    fn parse_cpu_str(content: &str) -> Vec<u32> {
        content
            .trim()
            .split(',')
            .filter_map(|cpu| {
                if cpu.contains('-') {
                    let (left, right) = cpu.split_once('-')?;
                    let left = left.parse::<u32>().ok()?;
                    let right = right.parse::<u32>().ok()?;

                    if left > right {
                        return None;
                    }

                    Some((left..=right).collect::<Vec<_>>())
                } else {
                    Some(vec![cpu.parse::<u32>().ok()?])
                }
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    /// All CPUs that are not isolated.
    fn root_cpus() -> anyhow::Result<Vec<u32>> {
        let isolated = Self::isolated_cpus().unwrap_or_default();
        let cpus = Self::online_cpus()
            .unwrap_or_default()
            .into_iter()
            .filter(|cpu| !isolated.contains(cpu))
            .collect::<Vec<_>>();
        debug!("Root CPUs: {cpus:?}");

        Ok(cpus)
    }

    fn online_cpus() -> anyhow::Result<Vec<u32>> {
        let content = std::fs::read_to_string("/sys/devices/system/cpu/online")?;
        let cpus = Self::parse_cpu_str(&content);
        if cpus.is_empty() {
            return Err(anyhow::anyhow!("No online CPUs found"));
        }
        Ok(cpus)
    }

    fn isolated_cpus() -> anyhow::Result<Vec<u32>> {
        let content = std::fs::read_to_string("/sys/devices/system/cpu/isolated")?;
        let cpus = Self::parse_cpu_str(&content);
        if cpus.is_empty() {
            return Err(anyhow::anyhow!("No isolated CPUs found"));
        }
        Ok(cpus)
    }
}

impl Drop for CpuIsolation {
    fn drop(&mut self) {
        self.reset_isolation()
            .unwrap_or_else(|e| eprintln!("Failed to reset CPU isolation: {}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_str() {
        assert_eq!(CpuIsolation::parse_cpu_str("").len(), 0);
        assert_eq!(CpuIsolation::parse_cpu_str("0,1,2,3"), vec![0, 1, 2, 3]);
        assert_eq!(CpuIsolation::parse_cpu_str("0-1,2-3"), vec![0, 1, 2, 3]);
        assert_eq!(
            CpuIsolation::parse_cpu_str("0,2,4-7"),
            vec![0, 2, 4, 5, 6, 7]
        );
    }

    #[test]
    fn test_cpu_bindings() {
        let online_cpus = CpuIsolation::online_cpus().unwrap_or_default();
        assert!(!online_cpus.is_empty());

        let (perf_cpu, bench_cpu, system_cpus) = CpuIsolation::cpu_bindings();
        assert!(perf_cpu != bench_cpu);
        assert!(!system_cpus.is_empty());
        assert!(!system_cpus.contains(&bench_cpu));
    }
}
