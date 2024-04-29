#[cfg(not(test))]
pub fn find_repository_root() -> Option<String> {
    let path = std::env::current_dir().ok()?;
    let mut current_dir = std::path::Path::new(&path).canonicalize().ok()?;

    loop {
        let git_dir = current_dir.join(".git");
        if git_dir.exists() {
            return Some(current_dir.to_string_lossy().to_string());
        }

        if !current_dir.pop() {
            break;
        }
    }

    log::debug!("Could not find repository root");

    None
}

#[cfg(test)]
pub fn find_repository_root() -> Option<String> {
    None
}
