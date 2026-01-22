//! Shared constants for the exec-harness crate.
//!
//! These constants are defined in the build script (build.rs) and exported as
//! environment variables. The same values are passed to the C preload library
//! as compiler defines, ensuring both Rust and C code use the same source of truth.

/// Environment variable name for the benchmark URI.
pub const URI_ENV: &str = env!("CODSPEED_URI_ENV");

/// Integration name reported to CodSpeed.
pub const INTEGRATION_NAME: &str = env!("CODSPEED_INTEGRATION_NAME");

/// Integration version reported to CodSpeed.
/// This should match the version of the `codspeed` crate dependency.
pub const INTEGRATION_VERSION: &str = env!("CODSPEED_INTEGRATION_VERSION");
