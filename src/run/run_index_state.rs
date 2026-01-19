use crate::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Manages a counter file to track upload index within a CI job.
///
/// This is used to differentiate multiple uploads in the same CI job execution
/// (e.g., running both simulation and memory benchmarks in the same job).
///
/// State is stored at: `{repository_root}/.codspeed/run-state/{run_id}/{run_part_id_hash}.json`
///
/// When a job is retried, it gets a fresh environment, so the counter resets to 0,
/// which ensures the `run_part_id` remains the same for each upload position.
pub struct RunIndexState {
    state_file_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct StateFile {
    #[serde(default)]
    run_index: u32,
}

impl RunIndexState {
    /// Creates a new `RunIndexState` for the given run and run part.
    ///
    /// # Arguments
    /// * `repository_root_path` - The root path of the repository
    /// * `run_id` - The CI run identifier (e.g., GitHub Actions run ID)
    /// * `run_part_id` - The run part identifier (job name + matrix info)
    pub fn new(repository_root_path: &str, run_id: &str, run_part_id: &str) -> Self {
        // Hash the run_part_id to avoid filesystem-unsafe characters
        // (run_part_id can contain JSON with colons, braces, quotes, etc.)
        let run_part_id_hash = sha256::digest(run_part_id);
        let state_file_path = PathBuf::from(repository_root_path)
            .join(".codspeed")
            .join("run-state")
            .join(run_id)
            .join(format!("{run_part_id_hash}.json"));

        Self { state_file_path }
    }

    /// Returns the current index and increments it for the next call.
    ///
    /// If the state file doesn't exist, starts at 0.
    /// The incremented value is persisted for subsequent calls.
    pub fn get_and_increment(&self) -> Result<u32> {
        // Create parent directories if needed
        if let Some(parent) = self.state_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read current state (default to empty if file doesn't exist)
        let mut state: StateFile = if self.state_file_path.exists() {
            let content = fs::read_to_string(&self.state_file_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            StateFile::default()
        };

        let current = state.run_index;

        // Update and write back
        state.run_index = current + 1;
        fs::write(&self.state_file_path, serde_json::to_string_pretty(&state)?)?;

        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_and_increment_starts_at_zero() {
        let temp_dir = TempDir::new().unwrap();
        let state = RunIndexState::new(
            temp_dir.path().to_str().unwrap(),
            "run-123",
            "my_job-{\"shard\":1}",
        );

        assert_eq!(state.get_and_increment().unwrap(), 0);
    }

    #[test]
    fn test_get_and_increment_increments() {
        let temp_dir = TempDir::new().unwrap();
        let state = RunIndexState::new(
            temp_dir.path().to_str().unwrap(),
            "run-123",
            "my_job-{\"shard\":1}",
        );

        assert_eq!(state.get_and_increment().unwrap(), 0);
        assert_eq!(state.get_and_increment().unwrap(), 1);
        assert_eq!(state.get_and_increment().unwrap(), 2);
    }

    #[test]
    fn test_different_run_part_ids_have_separate_counters() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        let state1 = RunIndexState::new(repo_path, "run-123", "job_a");
        let state2 = RunIndexState::new(repo_path, "run-123", "job_b");

        assert_eq!(state1.get_and_increment().unwrap(), 0);
        assert_eq!(state2.get_and_increment().unwrap(), 0);
        assert_eq!(state1.get_and_increment().unwrap(), 1);
        assert_eq!(state2.get_and_increment().unwrap(), 1);
    }

    #[test]
    fn test_different_run_ids_have_separate_counters() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        let state1 = RunIndexState::new(repo_path, "run-123", "my_job");
        let state2 = RunIndexState::new(repo_path, "run-456", "my_job");

        assert_eq!(state1.get_and_increment().unwrap(), 0);
        assert_eq!(state2.get_and_increment().unwrap(), 0);
        assert_eq!(state1.get_and_increment().unwrap(), 1);
        assert_eq!(state2.get_and_increment().unwrap(), 1);
    }

    #[test]
    fn test_state_persists_across_new_instances() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        {
            let state = RunIndexState::new(repo_path, "run-123", "my_job");
            assert_eq!(state.get_and_increment().unwrap(), 0);
        }

        {
            let state = RunIndexState::new(repo_path, "run-123", "my_job");
            assert_eq!(state.get_and_increment().unwrap(), 1);
        }
    }

    #[test]
    fn test_creates_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        let state = RunIndexState::new(repo_path, "run-123", "my_job");
        state.get_and_increment().unwrap();

        // Verify the directory structure was created
        let codspeed_dir = temp_dir.path().join(".codspeed");
        assert!(codspeed_dir.exists());
        assert!(codspeed_dir.join("run-state").exists());
        assert!(codspeed_dir.join("run-state").join("run-123").exists());
    }
}
