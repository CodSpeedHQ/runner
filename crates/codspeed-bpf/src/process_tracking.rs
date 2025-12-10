use anyhow::{Context, Result};
use libbpf_rs::MapCore;

/// Bump memlock rlimit to allow BPF ring buffer allocation
///
/// BPF programs require the RLIMIT_MEMLOCK resource limit to be increased
/// to infinity to allow allocation of ring buffers. This function sets that limit.
pub fn bump_memlock_rlimit() -> Result<()> {
    let rlimit = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };

    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) };
    if ret != 0 {
        anyhow::bail!("Failed to increase rlimit");
    }

    Ok(())
}

/// Trait for BPF programs that implement process tracking
pub trait ProcessTracking {
    /// Get the tracked_pids map
    fn tracked_pids_map(&self) -> &impl MapCore;

    /// Add a PID to track
    fn add_tracked_pid(&self, pid: i32) -> Result<()> {
        self.tracked_pids_map()
            .update(
                &pid.to_le_bytes(),
                &1u8.to_le_bytes(),
                libbpf_rs::MapFlags::ANY,
            )
            .context("Failed to add PID to tracked set")?;
        Ok(())
    }
}
