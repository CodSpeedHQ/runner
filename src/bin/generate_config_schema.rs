//! Generates JSON Schema for codspeed.yaml configuration file
//!
//! Run with:
//! ```
//! cargo run --no-default-features --bin generate-config-schema
//! ```

use std::fs;

use codspeed_runner::ProjectConfig;
use schemars::schema_for;

const OUTPUT_FILE: &str = "schemas/codspeed.schema.json";

fn main() {
    let schema = schema_for!(ProjectConfig);
    let schema_json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");
    let output_file_path = std::path::Path::new(OUTPUT_FILE);
    fs::create_dir_all(output_file_path.parent().unwrap())
        .expect("Failed to create schemas directory");
    fs::write(OUTPUT_FILE, format!("{schema_json}\n")).expect("Failed to write schema file");
    println!("Schema written to {OUTPUT_FILE}");
}
