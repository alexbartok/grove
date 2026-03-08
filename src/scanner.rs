use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Options controlling the filesystem scan.
#[derive(Clone)]
pub struct ScanOptions {
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    pub cross_filesystems: bool,
}

/// Scan a directory tree for git repositories.
/// Returns paths to directories containing `.git`.
pub fn scan_repos(root: &Path, opts: &ScanOptions) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let root_dev = root_device_id(root);
    walk(root, opts, 0, root_dev, &mut repos);
    repos
}

fn walk(
    dir: &Path,
    opts: &ScanOptions,
    depth: usize,
    root_dev: Option<u64>,
    repos: &mut Vec<PathBuf>,
) {
    // Check if this directory is a git repo
    if dir.join(".git").exists() {
        repos.push(dir.to_path_buf());
        return; // Don't descend into repos
    }

    // Check depth limit
    if let Some(max) = opts.max_depth
        && depth >= max {
            return;
        }

    // Read directory entries
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip hidden directories unless requested
        if name.starts_with('.') && !opts.include_hidden {
            continue;
        }

        // Check filesystem boundary
        if !opts.cross_filesystems
            && let Some(root_dev) = root_dev
                && device_id(&path) != Some(root_dev) {
                    continue;
                }

        walk(&path, opts, depth + 1, root_dev, repos);
    }
}

#[cfg(unix)]
fn device_id(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|m| m.dev())
}

#[cfg(unix)]
fn root_device_id(path: &Path) -> Option<u64> {
    device_id(path)
}

#[cfg(not(unix))]
fn device_id(_path: &Path) -> Option<u64> {
    None
}

#[cfg(not(unix))]
fn root_device_id(_path: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_fake_repo(dir: &Path) {
        fs::create_dir_all(dir.join(".git")).unwrap();
    }

    #[test]
    fn test_finds_repo_in_root() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(tmp.path());
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], tmp.path());
    }

    #[test]
    fn test_finds_nested_repos() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join("a"));
        create_fake_repo(&tmp.path().join("b/c"));
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_does_not_descend_into_repo() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        create_fake_repo(&parent);
        create_fake_repo(&parent.join("sub"));
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], parent);
    }

    #[test]
    fn test_skips_hidden_dirs_by_default() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join(".hidden_repo"));
        create_fake_repo(&tmp.path().join("visible"));
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], tmp.path().join("visible"));
    }

    #[test]
    fn test_includes_hidden_dirs_when_requested() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join(".hidden_repo"));
        create_fake_repo(&tmp.path().join("visible"));
        let opts = ScanOptions {
            include_hidden: true,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_max_depth_limits_traversal() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join("a")); // depth 1
        create_fake_repo(&tmp.path().join("b/c")); // depth 2
        create_fake_repo(&tmp.path().join("d/e/f")); // depth 3
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: Some(2),
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_root_is_repo() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(tmp.path());
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
    }
}
