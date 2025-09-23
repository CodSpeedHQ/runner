use crate::prelude::*;
use anyhow::anyhow;
use std::env;

pub fn get_env_variable(name: &str) -> Result<String> {
    env::var(name).map_err(|_| anyhow!("{name} environment variable not found"))
}

#[cfg(test)]
mod tests {
    use temp_env::with_var;

    use super::*;

    #[test]
    fn test_get_env_variable() {
        with_var("MY_CUSTOM_ENV_VAR", Some("test"), || {
            let env = get_env_variable("MY_CUSTOM_ENV_VAR").unwrap();
            assert_eq!(env, "test");
        });
    }

    #[test]
    fn test_get_env_variable_not_found() {
        let result = get_env_variable("MY_CUSTOM_ENV_VAR_NOT_FOUND");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "MY_CUSTOM_ENV_VAR_NOT_FOUND environment variable not found"
        );
    }
}
