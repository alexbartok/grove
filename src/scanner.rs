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

/// Progress information emitted during scanning.
pub struct ScanProgress<'a> {
    pub dirs_scanned: usize,
    pub repos_found: usize,
    pub current_dir: &'a Path,
}

/// Scan a directory tree for git repositories.
/// Returns paths to directories containing `.git`.
pub fn scan_repos(root: &Path, opts: &ScanOptions) -> Vec<PathBuf> {
    scan_repos_with_progress(root, opts, |_| {})
}

/// Scan with a progress callback invoked for each directory visited.
pub fn scan_repos_with_progress(
    root: &Path,
    opts: &ScanOptions,
    mut on_progress: impl FnMut(&ScanProgress),
) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let mut dirs_scanned = 0_usize;
    let root_dev = root_device_id(root);
    walk(root, opts, 0, root_dev, &mut repos, &mut dirs_scanned, &mut on_progress);
    repos
}

fn walk(
    dir: &Path,
    opts: &ScanOptions,
    depth: usize,
    root_dev: Option<u64>,
    repos: &mut Vec<PathBuf>,
    dirs_scanned: &mut usize,
    on_progress: &mut impl FnMut(&ScanProgress),
) {
    *dirs_scanned += 1;

    // Check if this directory is a git repo
    if dir.join(".git").exists() {
        repos.push(dir.to_path_buf());
        on_progress(&ScanProgress {
            dirs_scanned: *dirs_scanned,
            repos_found: repos.len(),
            current_dir: dir,
        });
        return; // Don't descend into repos
    }

    on_progress(&ScanProgress {
        dirs_scanned: *dirs_scanned,
        repos_found: repos.len(),
        current_dir: dir,
    });

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
        // Use file_type() from DirEntry to avoid extra stat syscall
        let is_dir = match entry.file_type() {
            Ok(ft) => ft.is_dir(),
            Err(_) => continue,
        };

        if !is_dir {
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

        let path = entry.path();

        // Check filesystem boundary
        if !opts.cross_filesystems
            && let Some(root_dev) = root_dev
                && device_id(&path) != Some(root_dev) {
                    continue;
                }

        walk(&path, opts, depth + 1, root_dev, repos, dirs_scanned, on_progress);
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

    #[test]
    fn test_progress_callback() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join("a"));
        create_fake_repo(&tmp.path().join("b"));
        fs::create_dir_all(tmp.path().join("empty")).unwrap();
        let opts = ScanOptions {
            include_hidden: false,
            max_depth: None,
            cross_filesystems: true,
        };
        let mut calls = 0_usize;
        let mut last_repos_found = 0_usize;
        scan_repos_with_progress(tmp.path(), &opts, |p| {
            calls += 1;
            last_repos_found = p.repos_found;
        });
        // root + a + b + empty = 4 dirs visited
        assert_eq!(calls, 4);
        assert_eq!(last_repos_found, 2);
    }
}
