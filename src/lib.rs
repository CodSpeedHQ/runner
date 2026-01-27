//! CodSpeed Runner library

macro_rules! cfg_full {
    ($(mod $name:ident;)*) => {
        $(#[cfg(feature = "full")] mod $name;)*
    };
    ($(pub mod $name:ident;)*) => {
        $(#[cfg(feature = "full")] pub mod $name;)*
    };
}

mod project_config;
pub use project_config::ProjectConfig;

cfg_full! {
    mod api_client;
    mod binary_installer;
    mod config;
    mod executor;
    mod instruments;
    mod local_logger;
    mod prelude;
    mod request_client;
    mod run_environment;
    mod runner_mode;
    mod system;
    mod upload;
}

cfg_full! {
    pub mod cli;
    pub mod logger;
}

#[cfg(feature = "full")]
pub use local_logger::clean_logger;
#[cfg(feature = "full")]
pub use runner_mode::RunnerMode;

#[cfg(feature = "full")]
mod full_consts {
    use lazy_static::lazy_static;
    use semver::Version;

    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const MONGODB_TRACER_VERSION: &str = "cs-mongo-tracer-v0.2.0";
    pub const VALGRIND_CODSPEED_VERSION: Version = Version::new(3, 26, 0);
    pub const VALGRIND_CODSPEED_DEB_REVISION_SUFFIX: &str = "0codspeed0";

    lazy_static! {
        pub static ref VALGRIND_CODSPEED_VERSION_STRING: String =
            format!("{VALGRIND_CODSPEED_VERSION}.codspeed");
        pub static ref VALGRIND_CODSPEED_DEB_VERSION: String =
            format!("{VALGRIND_CODSPEED_VERSION}-{VALGRIND_CODSPEED_DEB_REVISION_SUFFIX}");
    }
}

#[cfg(feature = "full")]
pub use full_consts::*;
