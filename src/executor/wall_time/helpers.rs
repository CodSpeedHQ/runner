use crate::prelude::*;
use runner_shared::walltime_results::WalltimeResults;
use std::path::Path;

fn add_empty_result_error_explanation(error_details: &str) -> String {
    format!(
        "The command did not produce any CodSpeed result to process, did you run your benchmarks using a compatible CodSpeed integration?\n\
        Check out https://codspeed.io/docs/benchmarks/overview for more information.\n\n\
        Details: {error_details}"
    )
}

/// Validates that walltime results exist and contain at least one benchmark.
/// When `allow_empty` is true, empty benchmark results are allowed.
pub fn validate_walltime_results(profile_folder: &Path, allow_empty: bool) -> Result<()> {
    let results_dir = profile_folder.join("results");

    if !results_dir.exists() {
        if allow_empty {
            warn!("No walltime results found in profile folder: {results_dir:?}.");
            return Ok(());
        }
        bail!(add_empty_result_error_explanation(&format!(
            "No walltime results found in profile folder: {results_dir:?}."
        )));
    }

    debug!("Validating walltime results in {results_dir:?}");

    let mut found_benchmark_results = false;

    for entry in std::fs::read_dir(&results_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process JSON files
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        debug!("Parsing walltime results from {path:?}");
        let file = std::fs::File::open(&path)
            .with_context(|| format!("Failed to open walltime results file: {path:?}"))?;

        let results: WalltimeResults = serde_json::from_reader(&file)
            .with_context(|| format!("Failed to parse walltime results from: {path:?}"))?;

        if results.benchmarks.is_empty() {
            if !allow_empty {
                bail!(add_empty_result_error_explanation(&format!(
                    "No benchmarks found in walltime results file: {path:?}."
                )));
            }
            debug!("No benchmarks found in {path:?} (allowed)");
        }

        found_benchmark_results = true;
        debug!(
            "Found {} benchmark(s) in {path:?}",
            results.benchmarks.len()
        );
    }

    if !found_benchmark_results {
        if allow_empty {
            warn!("No JSON result files found in: {results_dir:?}.");
            return Ok(());
        }
        bail!(add_empty_result_error_explanation(&format!(
            "No JSON result files found in: {results_dir:?}."
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Test helpers
    struct TestProfileFolder {
        temp_dir: TempDir,
    }

    impl TestProfileFolder {
        fn new() -> Self {
            Self {
                temp_dir: TempDir::new().unwrap(),
            }
        }

        fn path(&self) -> &Path {
            self.temp_dir.path()
        }

        fn results_dir(&self) -> std::path::PathBuf {
            self.path().join("results")
        }

        fn create_results_dir(&self) {
            fs::create_dir_all(self.results_dir()).unwrap();
        }

        fn write_json_file(&self, filename: &str, content: &str) {
            self.create_results_dir();
            let file_path = self.results_dir().join(filename);
            let mut file = fs::File::create(file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }

        fn write_text_file(&self, filename: &str, content: &str) {
            self.create_results_dir();
            let file_path = self.results_dir().join(filename);
            let mut file = fs::File::create(file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }
    }

    fn valid_walltime_results_json(benchmark_count: usize) -> String {
        let benchmarks: Vec<String> = (0..benchmark_count)
            .map(|i| {
                format!(
                    r#"{{
                        "name": "bench_{i}",
                        "uri": "test.rs::bench_{i}",
                        "config": {{}},
                        "stats": {{
                            "min_ns": 100.0,
                            "max_ns": 200.0,
                            "mean_ns": 150.0,
                            "stdev_ns": 10.0,
                            "q1_ns": 140.0,
                            "median_ns": 150.0,
                            "q3_ns": 160.0,
                            "rounds": 100,
                            "total_time": 15000.0,
                            "iqr_outlier_rounds": 0,
                            "stdev_outlier_rounds": 0,
                            "iter_per_round": 1,
                            "warmup_iters": 10
                        }}
                    }}"#
                )
            })
            .collect();

        format!(
            r#"{{
                "creator": {{
                    "name": "test",
                    "version": "1.0.0",
                    "pid": 12345
                }},
                "instrument": {{
                    "type": "walltime"
                }},
                "benchmarks": [{}]
            }}"#,
            benchmarks.join(",")
        )
    }

    fn empty_benchmarks_json() -> String {
        r#"{
            "creator": {
                "name": "test",
                "version": "1.0.0",
                "pid": 12345
            },
            "instrument": {
                "type": "walltime"
            },
            "benchmarks": []
        }"#
        .to_string()
    }

    // Success cases

    #[test]
    fn test_valid_single_result_file() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results.json", &valid_walltime_results_json(1));

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_multiple_result_files() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results1.json", &valid_walltime_results_json(2));
        profile.write_json_file("results2.json", &valid_walltime_results_json(3));

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ignores_non_json_files() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results.json", &valid_walltime_results_json(1));
        profile.write_text_file("readme.txt", "This is a text file");
        profile.write_text_file("data.csv", "col1,col2");

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_ok());
    }

    // Failure cases

    #[test]
    fn test_missing_results_directory() {
        let profile = TestProfileFolder::new();
        // Don't create results directory

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("No walltime results found in profile folder"));
    }

    #[test]
    fn test_empty_results_directory() {
        let profile = TestProfileFolder::new();
        profile.create_results_dir();

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("No JSON result files found in"));
    }

    #[test]
    fn test_no_json_files_in_directory() {
        let profile = TestProfileFolder::new();
        profile.write_text_file("readme.txt", "some text");
        profile.write_text_file("data.csv", "col1,col2");

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("No JSON result files found in"));
    }

    #[test]
    fn test_empty_benchmarks_array() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results.json", &empty_benchmarks_json());

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("No benchmarks found in walltime results file"));
    }

    #[test]
    fn test_invalid_json_format() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results.json", "{ invalid json }");

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Failed to parse walltime results from"));
    }

    #[test]
    fn test_multiple_files_one_empty() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results1.json", &valid_walltime_results_json(2));
        profile.write_json_file("results2.json", &empty_benchmarks_json());

        let result = validate_walltime_results(profile.path(), false);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("No benchmarks found in walltime results file"));
    }

    // Allow empty cases

    #[test]
    fn test_allow_empty_with_empty_benchmarks() {
        let profile = TestProfileFolder::new();
        profile.write_json_file("results.json", &empty_benchmarks_json());

        let result = validate_walltime_results(profile.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_allow_empty_with_missing_results_directory() {
        let profile = TestProfileFolder::new();
        // Don't create results directory

        let result = validate_walltime_results(profile.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_allow_empty_with_no_json_files() {
        let profile = TestProfileFolder::new();
        profile.create_results_dir();

        let result = validate_walltime_results(profile.path(), true);
        assert!(result.is_ok());
    }
}
