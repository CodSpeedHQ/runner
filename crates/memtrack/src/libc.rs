#[cfg(feature = "ebpf")]
pub fn find_libc_paths() -> anyhow::Result<Vec<std::path::PathBuf>> {
    use itertools::Itertools;

    let patterns = [
        // Debian, Ubuntu: Standard Linux multiarch paths
        "/lib/*-linux-gnu/libc.so.6",
        "/usr/lib/*-linux-gnu/libc.so.6",
        // RHEL, Fedora, CentOS, Arch:
        "/lib*/libc.so.6",
        "/usr/lib*/libc.so.6",
        // NixOS: find all glibc versions in the Nix store
        "/nix/store/*glibc*/lib/libc.so.6",
    ];

    let existing_paths = patterns
        .iter()
        .flat_map(|pattern| glob::glob(pattern).ok())
        .flatten()
        .filter_map(|p| p.ok())
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
