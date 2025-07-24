use std::collections::HashMap;

use crate::prelude::*;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTestOutput {
    #[serde(rename = "Time")]
    pub time: Option<String>,
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Package")]
    pub package: String,
    #[serde(rename = "Test")]
    pub test: Option<String>,
    #[serde(rename = "Output")]
    pub output: Option<String>,
    #[serde(rename = "Elapsed")]
    pub elapsed: Option<f64>,
}

pub struct RawOutput {
    name: String,
    time: f64,
    iters: u64,
}

impl RawOutput {
    fn parse_output(line: &str) -> Result<Option<RawOutput>> {
        lazy_static::lazy_static! {
            static ref BENCHMARK_REGEX: Regex = Regex::new(
                r"^(Benchmark[\w/]+)(?:-\d+)?\s+(\d+)\s+([0-9.]+)\s*ns/op"
            ).unwrap();
        }

        if let Some(captures) = BENCHMARK_REGEX.captures(line.trim()) {
            let name = captures
                .get(1)
                .context("Failed to get benchmark name")?
                .as_str()
                .to_string();
            let iters: u64 = captures
                .get(2)
                .context("Failed to get iterations")?
                .as_str()
                .parse()?;
            let time: f64 = captures
                .get(3)
                .context("Failed to get time")?
                .as_str()
                .parse()?;

            Ok(Some(RawOutput { name, time, iters }))
        } else {
            Ok(None)
        }
    }

    pub fn parse(output: &str) -> Result<Vec<(String, Self)>> {
        let mut results = Vec::new();
        for line in output.lines() {
            let event: RawTestOutput = serde_json::from_str(line)?;

            if event.action != "output" {
                continue;
            }
            let Some(output_text) = &event.output else {
                continue;
            };
            let Some(measurement) = Self::parse_output(output_text)? else {
                continue;
            };

            results.push((event.package, measurement));
        }

        Ok(results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkData {
    pub package: String,
    pub name: String,
    pub times: Vec<f64>,
    pub iters: Vec<u64>,
}

impl BenchmarkData {
    pub fn process_raw_results(raw_results: Vec<(String, RawOutput)>) -> Vec<Self> {
        let grouped: HashMap<(String, String), Vec<&(String, RawOutput)>> = raw_results
            .iter()
            .into_group_map_by(|(package, bench)| (package.clone(), bench.name.clone()));

        grouped
            .into_iter()
            .map(|((package, name), measurements)| BenchmarkData {
                package,
                name,
                // WalltimeResults expects times to be the _total time_ for each round.
                times: measurements
                    .iter()
                    .map(|(_, m)| m.time * m.iters as f64)
                    .collect(),
                iters: measurements.iter().map(|(_, m)| m.iters).collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output() {
        let line = "BenchmarkFibonacci10-16    \t   15564\t       755.2 ns/op\n";

        let result = RawOutput::parse_output(line).unwrap().unwrap();
        assert_eq!(result.name, "BenchmarkFibonacci10");
        assert_eq!(result.iters, 15564);
        assert!((result.time - 755.2).abs() < 0.1);

        let line =
            "BenchmarkOutTransform/pointer_to_value-16        \t 8257989\t       162.7 ns/op\n";
        let result = RawOutput::parse_output(line).unwrap().unwrap();
        assert_eq!(result.name, "BenchmarkOutTransform/pointer_to_value");
        assert_eq!(result.iters, 8257989);
        assert!((result.time - 162.7).abs() < 0.1);
    }

    #[test]
    fn test_parse_output_no_match() {
        // Test line that doesn't match benchmark pattern
        let line = "=== RUN   BenchmarkFibonacci10\n";

        let result = RawOutput::parse_output(line).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_and_process_benchmark_data() {
        const RESULT: &str = include_str!("testdata/simple.txt");

        let raw_results = RawOutput::parse(RESULT).unwrap();
        let processed = BenchmarkData::process_raw_results(raw_results);
        assert_eq!(processed.len(), 3);

        let fib10 = processed
            .iter()
            .find(|b| b.name == "BenchmarkFibonacci10")
            .unwrap();

        let fib20 = processed
            .iter()
            .find(|b| b.name == "BenchmarkFibonacci20")
            .unwrap();

        let fib30 = processed
            .iter()
            .find(|b| b.name == "BenchmarkFibonacci30")
            .unwrap();

        assert_eq!(fib10.package, "example");
        assert_eq!(fib10.times.len(), 10);
        assert_eq!(fib10.iters.len(), 10);
        assert_eq!(fib20.package, "example");
        assert_eq!(fib20.times.len(), 10);
        assert_eq!(fib20.iters.len(), 10);
        assert_eq!(fib30.package, "example");
        assert_eq!(fib30.times.len(), 10);
        assert_eq!(fib30.iters.len(), 10);
    }

    #[test]
    fn test_parse_fuego() {
        let content = include_str!("testdata/fuego.txt");
        let raw_results = RawOutput::parse(content).unwrap();
        let processed = BenchmarkData::process_raw_results(raw_results);
        assert_eq!(processed.len(), 19);
    }
}
