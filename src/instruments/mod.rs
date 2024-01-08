use serde::{Deserialize, Serialize};

use crate::config::Config;

pub mod mongo_tracer;

#[derive(Debug, PartialEq, Eq)]
pub struct MongoDBConfig {
    pub uri_env_name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Instruments {
    pub mongodb: Option<MongoDBConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum InstrumentNames {
    MongoDB,
}

impl Instruments {
    pub fn is_mongodb_enabled(&self) -> bool {
        self.mongodb.is_some()
    }

    pub fn get_active_instrument_names(&self) -> Vec<InstrumentNames> {
        let mut names = vec![];

        if self.is_mongodb_enabled() {
            names.push(InstrumentNames::MongoDB);
        }

        names
    }
}

impl From<&Config> for Instruments {
    fn from(config: &Config) -> Self {
        let mongodb = config
            .mongo_uri_env_name
            .as_ref()
            .map(|uri_env_name| MongoDBConfig {
                uri_env_name: uri_env_name.clone(),
            });

        Self { mongodb }
    }
}

#[cfg(test)]
impl Instruments {
    /// Constructs a new `Instruments` with default values for testing purposes
    pub fn test() -> Self {
        Self {
            mongodb: Some(MongoDBConfig {
                uri_env_name: "MONGODB_URI".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use temp_env::with_var;

    use super::*;

    #[test]
    fn test_from_env_empty() {
        let instruments = Instruments::from(&Config::test());
        assert!(instruments.mongodb.is_none());
    }

    #[test]
    fn test_from_env_mongodb() {
        with_var(
            "CODSPEED_MONGO_INSTR_URI_ENV_NAME",
            Some("MONGODB_URI"),
            || {
                let config = Config {
                    mongo_uri_env_name: Some("MONGODB_URI".into()),
                    ..Config::test()
                };
                let instruments = Instruments::from(&config);
                assert_eq!(
                    instruments.mongodb,
                    Some(MongoDBConfig {
                        uri_env_name: "MONGODB_URI".into()
                    })
                );

                assert!(instruments.is_mongodb_enabled());
            },
        );
    }
}
