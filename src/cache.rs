use std::path::{Path, PathBuf};

fn cache_dir() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
        })
        .join("grove")
}

/// FNV-1a hash — stable across Rust versions (unlike DefaultHasher).
fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn cache_file(scan_path: &Path) -> PathBuf {
    let hash = stable_hash(scan_path.as_os_str().as_encoded_bytes());
    cache_dir().join(format!("{:016x}.paths", hash))
}

/// Load cached repo paths for the given scan root.
/// Returns None if no cache exists or it's invalid.
pub fn load(scan_path: &Path) -> Option<Vec<PathBuf>> {
    let file = cache_file(scan_path);
    let content = std::fs::read_to_string(file).ok()?;
    let mut lines = content.lines();

    let cached_root = lines.next()?;
    if Path::new(cached_root) != scan_path {
        return None;
    }

    let paths: Vec<PathBuf> = lines.map(PathBuf::from).collect();
    if paths.is_empty() {
        return None;
    }
    Some(paths)
}

/// Save repo paths to the cache for the given scan root.
pub fn save(scan_path: &Path, repo_paths: &[PathBuf]) {
    let file = cache_file(scan_path);
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut content = scan_path.display().to_string();
    for path in repo_paths {
        content.push('\n');
        content.push_str(&path.display().to_string());
    }

    let _ = std::fs::write(file, content);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn cache_file_is_deterministic() {
        let path = Path::new("/home/user/projects");
        let a = cache_file(path);
        let b = cache_file(path);
        assert_eq!(a, b);
    }

    #[test]
    fn different_paths_produce_different_cache_files() {
        let a = cache_file(Path::new("/home/user/projects"));
        let b = cache_file(Path::new("/home/user/other"));
        assert_ne!(a, b);
    }

    #[test]
    fn load_returns_none_for_nonexistent_cache() {
        let result = load(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_none());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        // Use the temp dir itself as the scan_path — unique per test run
        let scan_path = tmp.path().to_path_buf();

        let repo_paths = vec![
            scan_path.join("repo-a"),
            scan_path.join("repo-b"),
            scan_path.join("sub/repo-c"),
        ];

        save(&scan_path, &repo_paths);
        let loaded = load(&scan_path).expect("should load cached paths");
        assert_eq!(loaded, repo_paths);
    }

    #[test]
    fn load_returns_none_when_scan_path_mismatches() {
        let tmp = tempfile::tempdir().unwrap();
        let scan_path = tmp.path().to_path_buf();
        let _other_path = tmp.path().join("other");

        let repo_paths = vec![scan_path.join("repo-a")];
        save(&scan_path, &repo_paths);

        // Manually read the cache file and rewrite with a different root
        let file = cache_file(&scan_path);
        let content = fs::read_to_string(&file).unwrap();
        let tampered = content.replacen(&scan_path.display().to_string(), "/wrong/root", 1);
        fs::write(&file, tampered).unwrap();

        let result = load(&scan_path);
        assert!(result.is_none());
    }

    #[test]
    fn load_returns_none_for_empty_repo_list() {
        let tmp = tempfile::tempdir().unwrap();
        let scan_path = tmp.path().to_path_buf();

        // Save with no repo paths
        save(&scan_path, &[]);

        let result = load(&scan_path);
        assert!(result.is_none());
    }
}
