use crate::prelude::*;

pub struct NameAndUri {
    pub(crate) name: String,
    pub(crate) uri: String,
    command: Vec<String>,
}

/// Maximum length for benchmark name to avoid excessively long URIs
/// Should be removed once we have structured metadata around benchmarks
const MAX_NAME_LENGTH: usize = 1024 - 100;

pub fn generate_name_and_uri(name: &Option<String>, command: &[String]) -> NameAndUri {
    let mut name = name.clone().unwrap_or_else(|| command.join(" "));
    let uri = format!("exec_harness::{name}");

    if name.len() > MAX_NAME_LENGTH {
        warn!(
            "Benchmark name is too long ({} characters). Truncating to {} characters.",
            name.len(),
            MAX_NAME_LENGTH
        );
        name.truncate(MAX_NAME_LENGTH);
    }

    NameAndUri {
        name,
        uri,
        command: command.to_vec(),
    }
}

impl NameAndUri {
    pub fn print_executing(&self) {
        info!("Executing: {}", self.name);
        debug!("Command: {:?}", self.command);
    }
}
