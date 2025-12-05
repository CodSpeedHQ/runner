#[cfg(feature = "ebpf")]
pub fn find_libc_paths() -> anyhow::Result<Vec<std::path::PathBuf>> {
    use itertools::Itertools;

    let mut paths = vec![
        "/lib/x86_64-linux-gnu/libc.so.6".into(),
        "/usr/lib/x86_64-linux-gnu/libc.so.6".into(),
        "/lib64/libc.so.6".into(),
        "/usr/lib64/libc.so.6".into(),
    ];

    // On NixOS, try to find all glibc versions in the Nix store
    if let Ok(entries) = std::fs::read_dir("/nix/store") {
        let nix_paths: Vec<_> = entries
            .filter_map(|entry| {
                let Ok(entry) = entry else { return None };

                let path = entry.path();
                let file_name = path.file_name()?;
                let name = file_name.to_string_lossy();

                // Look for glibc directories
                if name.contains("glibc") && !name.contains("locales") && !name.contains("iconv") {
                    return Some(path.join("lib").join("libc.so.6"));
                }
                None
            })
            .collect();

        paths.extend(nix_paths);
    }

    let existing_paths = paths
        .into_iter()
        .filter_map(|p| p.canonicalize().ok())
        .filter(|path| {
            let Ok(metadata) = std::fs::metadata(path) else {
                return false;
            };
            metadata.is_file()
        })
        .dedup()
        .collect::<Vec<_>>();

    if existing_paths.is_empty() {
        anyhow::bail!("Could not find libc.so.6");
    }

    Ok(existing_paths)
}
