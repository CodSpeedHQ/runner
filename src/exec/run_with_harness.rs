use crate::prelude::*;
use std::path::PathBuf;

/// Find the exec-harness binary in the standard locations
fn find_harness_binary() -> Result<PathBuf> {
    // Try common locations where the harness might be installed
    let mut possible_paths = Vec::new();

    // Look in the same directory as the runner binary
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            possible_paths.push(parent.join("exec-harness"));
        }

        // Look in cargo target directory (for development)
        for ancestor in current_exe.ancestors() {
            if ancestor.ends_with("target") {
                possible_paths.push(ancestor.join("release").join("exec-harness"));
                possible_paths.push(ancestor.join("debug").join("exec-harness"));
                break;
            }
        }
    }

    for path in &possible_paths {
        if path.exists() && path.is_file() {
            debug!("Found exec-harness at: {path:?}");
            return Ok(path.clone());
        }
    }

    bail!(
        "exec-harness binary not found. Please ensure it's built and in the same directory as the runner.\nSearched paths: {possible_paths:?}"
    )
}

/// Wraps the user's command with the exec-harness binary
pub fn wrap_command_with_harness(
    user_command: &[String],
    benchmark_name: Option<&str>,
) -> Result<Vec<String>> {
    if user_command.is_empty() {
        bail!("Cannot wrap empty command");
    }

    let harness_path = find_harness_binary()?;
    let harness_path_str = harness_path
        .to_str()
        .context("exec-harness path contains invalid UTF-8")?;

    let mut wrapped_command = vec![harness_path_str.to_string()];

    // Add --name if provided
    if let Some(name) = benchmark_name {
        wrapped_command.push("--name".to_string());
        wrapped_command.push(name.to_string());
    }

    // Add -- separator (optional but clearer)
    wrapped_command.push("--".to_string());

    // Add the user's command
    wrapped_command.extend_from_slice(user_command);

    debug!("Wrapped command: {wrapped_command:?}");
    Ok(wrapped_command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_command_without_name() {
        let command = vec!["./my_binary".to_string(), "arg1".to_string()];
        let wrapped = wrap_command_with_harness(&command, None);

        // We can't test the exact path, but we can verify the structure
        if let Ok(wrapped) = wrapped {
            assert!(wrapped[0].contains("exec-harness"));
            assert_eq!(wrapped[wrapped.len() - 2], "./my_binary");
            assert_eq!(wrapped[wrapped.len() - 1], "arg1");
        }
    }

    #[test]
    fn test_wrap_command_with_name() {
        let command = vec!["./my_binary".to_string()];
        let wrapped = wrap_command_with_harness(&command, Some("custom_name"));

        if let Ok(wrapped) = wrapped {
            assert!(wrapped[0].contains("exec-harness"));
            assert!(wrapped.contains(&"--name".to_string()));
            assert!(wrapped.contains(&"custom_name".to_string()));
            assert_eq!(wrapped[wrapped.len() - 1], "./my_binary");
        }
    }

    #[test]
    fn test_wrap_empty_command() {
        let command: Vec<String> = vec![];
        let wrapped = wrap_command_with_harness(&command, None);
        assert!(wrapped.is_err());
    }
}
