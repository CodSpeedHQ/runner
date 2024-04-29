#[cfg(not(test))]
pub fn find_repository_root() -> Option<String> {
    let path = std::env::current_dir().ok()?;
    let current_dir = std::path::Path::new(&path).canonicalize().ok()?;

    for ancestor in current_dir.ancestors() {
        let git_dir = ancestor.join(".git");
        if git_dir.exists() {
            let mut repository_root = ancestor.to_path_buf();
            // add a trailing slash to the path
            repository_root.push("");
            return Some(repository_root.to_string_lossy().to_string());
        }
    }

    log::debug!("Could not find repository root");

    None
}

#[cfg(test)]
pub fn find_repository_root() -> Option<String> {
    None
}
