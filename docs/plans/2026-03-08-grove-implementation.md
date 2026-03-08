# Grove Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers-extended-cc:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI that scans for git repos and shows their safety status — answering "if this machine died, would I lose work?"

**Architecture:** Scan-then-display pipeline: CLI args → filesystem walk → git inspection → risk classification → render (static columns or interactive TUI). All repos discovered and inspected before any output.

**Tech Stack:** Rust 2024 edition, clap (derive), git2, ratatui, crossterm, colored, anyhow. Dev: tempfile.

**Design doc:** `docs/plans/2026-03-08-grove-design.md`

---

### Task 1: Project Scaffolding

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/model.rs`
- Create: `src/scanner.rs`
- Create: `src/git.rs`
- Create: `src/static_output.rs`
- Create: `src/tui/mod.rs`
- Create: `src/tui/ui.rs`
- Create: `src/tui/actions.rs`

**Step 1: Add dependencies to Cargo.toml**

```toml
[package]
name = "grove"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
colored = "2"
crossterm = "0.28"
git2 = "0.19"
ratatui = "0.29"

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create empty module files**

Create each file with just enough to compile:

`src/lib.rs`:
```rust
pub mod git;
pub mod model;
pub mod scanner;
pub mod static_output;
pub mod tui;
```

`src/model.rs`, `src/scanner.rs`, `src/git.rs`, `src/static_output.rs`: empty files.

`src/tui/mod.rs`:
```rust
pub mod actions;
pub mod ui;
```

`src/tui/ui.rs`, `src/tui/actions.rs`: empty files.

`src/main.rs`:
```rust
fn main() {
    println!("Hello, world!");
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors (warnings about unused are fine).

**Step 4: Commit**

```bash
git add -A
git commit -m "Add dependencies and module structure"
```

---

### Task 2: Data Model

**Files:**
- Modify: `src/model.rs`

**Step 1: Write tests for the data model**

Add to `src/model.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_at_risk_uncommitted_changes() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 2,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: true,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_no_remote() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: false,
            remote_name: None,
            has_upstream: false,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_unpushed() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: true,
            ahead: 3,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_stashes() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: true,
            ahead: 0,
            behind: 0,
            stash_count: 2,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_no_upstream() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: false,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_warning_detached_head() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: None,
            is_detached: true,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: true,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::Warning);
    }

    #[test]
    fn test_risk_safe() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".into()),
            has_upstream: true,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        };
        assert_eq!(info.risk_level(), RiskLevel::Safe);
    }

    #[test]
    fn test_sorting_by_risk() {
        let safe = RepoInfo {
            path: "/tmp/safe".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        let warning = RepoInfo {
            path: "/tmp/warning".into(),
            branch: None,
            is_detached: true,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        let at_risk = RepoInfo {
            path: "/tmp/at_risk".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 3, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };

        let mut repos = vec![safe, warning, at_risk];
        repos.sort_by_key(|r| r.risk_level());
        assert_eq!(repos[0].risk_level(), RiskLevel::AtRisk);
        assert_eq!(repos[1].risk_level(), RiskLevel::Warning);
        assert_eq!(repos[2].risk_level(), RiskLevel::Safe);
    }

    #[test]
    fn test_status_summary_clean() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        assert_eq!(info.status_summary(), "✓ clean");
    }

    #[test]
    fn test_status_summary_modified() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 2, staged_count: 1, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        assert_eq!(info.status_summary(), "3 modified");
    }

    #[test]
    fn test_sync_summary_synced() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        assert_eq!(info.sync_summary(), "✓ synced");
    }

    #[test]
    fn test_sync_summary_no_remote() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: false, remote_name: None,
            has_upstream: false, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        assert_eq!(info.sync_summary(), "✗ no remote");
    }

    #[test]
    fn test_sync_summary_ahead() {
        let info = RepoInfo {
            path: "/tmp/repo".into(),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 3, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        };
        assert_eq!(info.sync_summary(), "↑3 ahead");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib model`
Expected: FAIL — types don't exist yet.

**Step 3: Implement the data model**

Add the types and methods to `src/model.rs` (above the test module):

```rust
use std::path::PathBuf;

/// Risk level for a repository — determines sort order and display color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    AtRisk,  // Red — data would be lost
    Warning, // Yellow — unusual state
    Safe,    // Green — fully backed up
}

/// Complete status snapshot of a git repository.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_detached: bool,
    pub modified_count: usize,
    pub staged_count: usize,
    pub untracked_count: usize,
    pub has_remote: bool,
    pub remote_name: Option<String>,
    pub has_upstream: bool,
    pub ahead: usize,
    pub behind: usize,
    pub stash_count: usize,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
}

impl RepoInfo {
    /// Classify the risk level based on whether local-only data exists.
    pub fn risk_level(&self) -> RiskLevel {
        // At Risk: any condition where data would be lost
        if self.modified_count > 0
            || self.staged_count > 0
            || self.untracked_count > 0
            || !self.has_remote
            || self.ahead > 0
            || self.stash_count > 0
            || (!self.has_upstream && !self.is_detached)
            || self.merge_in_progress
            || self.rebase_in_progress
        {
            return RiskLevel::AtRisk;
        }

        // Warning: detached HEAD
        if self.is_detached {
            return RiskLevel::Warning;
        }

        RiskLevel::Safe
    }

    /// Human-readable branch display name.
    pub fn branch_display(&self) -> &str {
        if self.is_detached {
            "HEAD"
        } else {
            self.branch.as_deref().unwrap_or("???")
        }
    }

    /// Working tree status summary for columnar display.
    pub fn status_summary(&self) -> String {
        let dirty = self.modified_count + self.staged_count;
        if self.merge_in_progress {
            return "⚠ merging".into();
        }
        if self.rebase_in_progress {
            return "⚠ rebasing".into();
        }
        if dirty > 0 && self.untracked_count > 0 {
            return format!("{dirty} modified, {} untracked", self.untracked_count);
        }
        if dirty > 0 {
            return format!("{dirty} modified");
        }
        if self.untracked_count > 0 {
            return format!("{} untracked", self.untracked_count);
        }
        "✓ clean".into()
    }

    /// Stash summary for columnar display.
    pub fn stash_summary(&self) -> String {
        if self.stash_count > 0 {
            self.stash_count.to_string()
        } else {
            "—".into()
        }
    }

    /// Remote sync summary for columnar display.
    pub fn sync_summary(&self) -> String {
        if !self.has_remote {
            return "✗ no remote".into();
        }
        if !self.has_upstream && !self.is_detached {
            return "⚠ no tracking".into();
        }
        if self.ahead > 0 && self.behind > 0 {
            return format!("↑{} ↓{}", self.ahead, self.behind);
        }
        if self.ahead > 0 {
            return format!("↑{} ahead", self.ahead);
        }
        if self.behind > 0 {
            return format!("↓{} behind", self.behind);
        }
        "✓ synced".into()
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib model`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/model.rs
git commit -m "Add data model with risk classification and display helpers"
```

---

### Task 3: CLI Argument Parsing

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement CLI parsing with clap**

Replace `src/main.rs`:

```rust
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Grove — would you lose work if this machine died?
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Directory to scan (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Static output mode (auto if not a TTY)
    #[arg(short = 'n', long = "no-interactive")]
    no_interactive: bool,

    /// Include hidden directories in traversal
    #[arg(short = 'H', long = "hidden")]
    hidden: bool,

    /// Maximum directory traversal depth
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Cross filesystem boundaries
    #[arg(long = "all-filesystems")]
    all_filesystems: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let interactive = !args.no_interactive && atty::is(atty::Stream::Stdout);

    // Canonicalize the path
    let scan_path = args.path.canonicalize()?;

    println!("Scanning: {}", scan_path.display());
    println!("Mode: {}", if interactive { "interactive" } else { "static" });

    Ok(())
}
```

Note: We need to add `atty` to dependencies for TTY detection.

**Step 2: Add atty dependency**

Add to `Cargo.toml` under `[dependencies]`:
```toml
atty = "0.2"
```

**Step 3: Verify it compiles and runs**

Run: `cargo run -- --help`
Expected: Shows help text with all options.

Run: `cargo run`
Expected: Prints scanning path and mode.

**Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "Add CLI argument parsing with clap"
```

---

### Task 4: Filesystem Scanner

**Files:**
- Modify: `src/scanner.rs`

**Step 1: Write tests for the scanner**

```rust
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Options controlling the filesystem scan.
pub struct ScanOptions {
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    pub cross_filesystems: bool,
}

/// Scan a directory tree for git repositories.
/// Returns paths to directories containing `.git`.
pub fn scan_repos(root: &Path, opts: &ScanOptions) -> Vec<PathBuf> {
    todo!()
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
        let opts = ScanOptions { include_hidden: false, max_depth: None, cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], tmp.path());
    }

    #[test]
    fn test_finds_nested_repos() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join("a"));
        create_fake_repo(&tmp.path().join("b/c"));
        let opts = ScanOptions { include_hidden: false, max_depth: None, cross_filesystems: true };
        let mut repos = scan_repos(tmp.path(), &opts);
        repos.sort();
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_does_not_descend_into_repo() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        create_fake_repo(&parent);
        // Nested repo inside parent — should NOT be found
        create_fake_repo(&parent.join("sub"));
        let opts = ScanOptions { include_hidden: false, max_depth: None, cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], parent);
    }

    #[test]
    fn test_skips_hidden_dirs_by_default() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join(".hidden_repo"));
        create_fake_repo(&tmp.path().join("visible"));
        let opts = ScanOptions { include_hidden: false, max_depth: None, cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], tmp.path().join("visible"));
    }

    #[test]
    fn test_includes_hidden_dirs_when_requested() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join(".hidden_repo"));
        create_fake_repo(&tmp.path().join("visible"));
        let opts = ScanOptions { include_hidden: true, max_depth: None, cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_max_depth_limits_traversal() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(&tmp.path().join("a"));           // depth 1
        create_fake_repo(&tmp.path().join("b/c"));         // depth 2
        create_fake_repo(&tmp.path().join("d/e/f"));       // depth 3
        let opts = ScanOptions { include_hidden: false, max_depth: Some(2), cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 2); // only depth 1 and 2
    }

    #[test]
    fn test_root_is_repo() {
        let tmp = TempDir::new().unwrap();
        create_fake_repo(tmp.path());
        let opts = ScanOptions { include_hidden: false, max_depth: None, cross_filesystems: true };
        let repos = scan_repos(tmp.path(), &opts);
        assert_eq!(repos.len(), 1);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib scanner`
Expected: FAIL — `todo!()` panics.

**Step 3: Implement the scanner**

Replace the `scan_repos` function:

```rust
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
    // Check if this directory itself is a git repo
    if dir.join(".git").exists() {
        repos.push(dir.to_path_buf());
        return; // Don't descend into repos
    }

    // Check depth limit
    if let Some(max) = opts.max_depth {
        if depth >= max {
            return;
        }
    }

    // Read directory entries
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return, // Permission denied, etc. — skip silently
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
        if !opts.cross_filesystems {
            if let Some(root_dev) = root_dev {
                if device_id(&path) != Some(root_dev) {
                    continue;
                }
            }
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib scanner`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/scanner.rs
git commit -m "Add filesystem scanner with hidden/depth/filesystem-boundary support"
```

---

### Task 5: Git Status Inspection

**Files:**
- Modify: `src/git.rs`

This is the most complex module. It uses `git2` to inspect each repo and produce a `RepoInfo`.

**Step 1: Write tests for git inspection**

These tests create real git repos in temp dirs using `git2`.

```rust
use std::path::Path;

use anyhow::Result;
use git2::Repository;

use crate::model::RepoInfo;

/// Inspect a git repository and return its full status.
pub fn inspect_repo(path: &Path) -> Result<RepoInfo> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Signature, Repository};
    use std::fs;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) -> Repository {
        Repository::init(dir).unwrap()
    }

    fn make_initial_commit(repo: &Repository) {
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    }

    #[test]
    fn test_clean_repo_no_remote() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        let info = inspect_repo(tmp.path()).unwrap();
        assert_eq!(info.branch.as_deref(), Some("main"));
        assert!(!info.is_detached);
        assert_eq!(info.modified_count, 0);
        assert_eq!(info.untracked_count, 0);
        assert!(!info.has_remote);
    }

    #[test]
    fn test_modified_files() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        // Create a file and commit it
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }
        make_initial_commit(&repo);
        // Modify the file
        fs::write(tmp.path().join("file.txt"), "changed").unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert_eq!(info.modified_count, 1);
    }

    #[test]
    fn test_untracked_files() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);
        fs::write(tmp.path().join("new_file.txt"), "hello").unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert_eq!(info.untracked_count, 1);
    }

    #[test]
    fn test_staged_files() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }
        make_initial_commit(&repo);
        // Modify and stage
        fs::write(tmp.path().join("file.txt"), "changed").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }

        let info = inspect_repo(tmp.path()).unwrap();
        assert_eq!(info.staged_count, 1);
    }

    #[test]
    fn test_stash_count() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }
        make_initial_commit(&repo);
        // Create a stash
        fs::write(tmp.path().join("file.txt"), "stashed").unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        repo.stash_save(&sig, "test stash", None).unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert_eq!(info.stash_count, 1);
    }

    #[test]
    fn test_empty_repo() {
        let tmp = TempDir::new().unwrap();
        let _repo = init_repo(tmp.path());
        // No commits at all
        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.branch.is_some()); // should still report branch name
        assert!(!info.has_remote);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib git`
Expected: FAIL — `todo!()`.

**Step 3: Implement git inspection**

```rust
pub fn inspect_repo(path: &Path) -> Result<RepoInfo> {
    let repo = Repository::open(path)?;

    let (branch, is_detached) = get_branch_info(&repo);
    let (modified_count, staged_count, untracked_count) = get_working_tree_status(&repo);
    let (has_remote, remote_name) = get_remote_info(&repo);
    let (has_upstream, ahead, behind) = get_upstream_info(&repo);
    let stash_count = get_stash_count(&repo);
    let merge_in_progress = path.join(".git/MERGE_HEAD").exists();
    let rebase_in_progress = path.join(".git/rebase-merge").exists()
        || path.join(".git/rebase-apply").exists();

    Ok(RepoInfo {
        path: path.to_path_buf(),
        branch,
        is_detached,
        modified_count,
        staged_count,
        untracked_count,
        has_remote,
        remote_name,
        has_upstream,
        ahead,
        behind,
        stash_count,
        merge_in_progress,
        rebase_in_progress,
    })
}

fn get_branch_info(repo: &Repository) -> (Option<String>, bool) {
    if repo.head_detached().unwrap_or(false) {
        return (None, true);
    }
    match repo.head() {
        Ok(head) => {
            let name = head.shorthand().map(String::from);
            (name, false)
        }
        Err(_) => {
            // Unborn branch (empty repo) — try to read the symbolic ref
            match repo.find_reference("HEAD") {
                Ok(head) => {
                    let target = head.symbolic_target().unwrap_or("refs/heads/main");
                    let name = target.strip_prefix("refs/heads/").unwrap_or(target);
                    (Some(name.to_string()), false)
                }
                Err(_) => (None, false),
            }
        }
    }
}

fn get_working_tree_status(repo: &Repository) -> (usize, usize, usize) {
    let statuses = match repo.statuses(Some(
        git2::StatusOptions::new()
            .include_untracked(true)
            .recurse_untracked_dirs(true),
    )) {
        Ok(s) => s,
        Err(_) => return (0, 0, 0),
    };

    let mut modified = 0;
    let mut staged = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let s = entry.status();
        if s.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        ) {
            modified += 1;
        }
        if s.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            staged += 1;
        }
        if s.intersects(git2::Status::WT_NEW) {
            untracked += 1;
        }
    }

    (modified, staged, untracked)
}

fn get_remote_info(repo: &Repository) -> (bool, Option<String>) {
    match repo.remotes() {
        Ok(remotes) => {
            if remotes.is_empty() {
                (false, None)
            } else {
                let name = remotes.get(0).map(String::from);
                (true, name)
            }
        }
        Err(_) => (false, None),
    }
}

fn get_upstream_info(repo: &Repository) -> (bool, usize, usize) {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return (false, 0, 0),
    };

    let local_branch_name = match head.shorthand() {
        Some(name) => name.to_string(),
        None => return (false, 0, 0),
    };

    let branch = match repo.find_branch(&local_branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return (false, 0, 0),
    };

    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return (false, 0, 0),
    };

    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return (true, 0, 0),
    };

    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return (true, 0, 0),
    };

    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((ahead, behind)) => (true, ahead, behind),
        Err(_) => (true, 0, 0),
    }
}

fn get_stash_count(repo: &Repository) -> usize {
    let mut count = 0;
    // stash_foreach requires &mut
    let repo_ptr = repo as *const Repository as *mut Repository;
    unsafe {
        let repo_mut = &mut *repo_ptr;
        let _ = repo_mut.stash_foreach(|_, _, _| {
            count += 1;
            true
        });
    }
    count
}
```

Note: The `get_stash_count` uses unsafe because `stash_foreach` requires `&mut Repository`. A cleaner alternative is to take `&mut Repository` in the public API, but since `inspect_repo` takes a path and opens the repo itself, we can just make the repo mutable. Adjust to:

```rust
pub fn inspect_repo(path: &Path) -> Result<RepoInfo> {
    let mut repo = Repository::open(path)?;
    // ... same as above but pass &mut repo to get_stash_count
    let stash_count = get_stash_count(&mut repo);
    // ...
}

fn get_stash_count(repo: &mut Repository) -> usize {
    let mut count = 0;
    let _ = repo.stash_foreach(|_, _, _| {
        count += 1;
        true
    });
    count
}
```

And adjust other functions to take `&Repository` (they already do).

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib git`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/git.rs
git commit -m "Add git status inspection with branch, status, remote, stash detection"
```

---

### Task 6: Static Output

**Files:**
- Modify: `src/static_output.rs`

**Step 1: Write tests**

```rust
use crate::model::RepoInfo;

/// Print repos as a colored columnar table to stdout.
pub fn print_static(repos: &[RepoInfo], home_dir: Option<&std::path::Path>) {
    todo!()
}

/// Format a single repo row (without color) for testing.
fn format_row(info: &RepoInfo, widths: &ColumnWidths, home_dir: Option<&std::path::Path>) -> String {
    todo!()
}

struct ColumnWidths {
    repo: usize,
    branch: usize,
    status: usize,
    stash: usize,
    remote: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RepoInfo;
    use std::path::PathBuf;

    fn make_safe_repo(path: &str) -> RepoInfo {
        RepoInfo {
            path: PathBuf::from(path),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0, staged_count: 0, untracked_count: 0,
            has_remote: true, remote_name: Some("origin".into()),
            has_upstream: true, ahead: 0, behind: 0,
            stash_count: 0, merge_in_progress: false, rebase_in_progress: false,
        }
    }

    #[test]
    fn test_tilde_contraction() {
        let info = make_safe_repo("/Users/alice/projects/foo");
        let home = PathBuf::from("/Users/alice");
        let widths = ColumnWidths { repo: 30, branch: 10, status: 10, stash: 5, remote: 10 };
        let row = format_row(&info, &widths, Some(&home));
        assert!(row.contains("~/projects/foo"));
    }

    #[test]
    fn test_column_alignment() {
        let info = make_safe_repo("/tmp/repo");
        let widths = ColumnWidths { repo: 20, branch: 10, status: 10, stash: 5, remote: 10 };
        let row = format_row(&info, &widths, None);
        // Should contain all columns
        assert!(row.contains("main"));
        assert!(row.contains("✓ clean"));
        assert!(row.contains("origin"));
        assert!(row.contains("✓ synced"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib static_output`
Expected: FAIL.

**Step 3: Implement static output**

```rust
use colored::Colorize;
use crate::model::{RepoInfo, RiskLevel};

struct ColumnWidths {
    repo: usize,
    branch: usize,
    status: usize,
    stash: usize,
    remote: usize,
}

pub fn print_static(repos: &[RepoInfo], home_dir: Option<&std::path::Path>) {
    if repos.is_empty() {
        println!("No git repositories found.");
        return;
    }

    let widths = compute_widths(repos, home_dir);

    // Header
    println!(
        "{:<rw$}  {:<bw$}  {:<sw$}  {:<tw$}  {:<mw$}  {}",
        "REPO", "BRANCH", "STATUS", "STASH", "REMOTE", "SYNC",
        rw = widths.repo, bw = widths.branch, sw = widths.status,
        tw = widths.stash, mw = widths.remote,
    );

    for info in repos {
        let row = format_row(info, &widths, home_dir);
        match info.risk_level() {
            RiskLevel::AtRisk => println!("{}", row.red()),
            RiskLevel::Warning => println!("{}", row.yellow()),
            RiskLevel::Safe => println!("{}", row.green()),
        }
    }
}

fn display_path(path: &std::path::Path, home_dir: Option<&std::path::Path>) -> String {
    if let Some(home) = home_dir {
        if let Ok(stripped) = path.strip_prefix(home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

fn compute_widths(repos: &[RepoInfo], home_dir: Option<&std::path::Path>) -> ColumnWidths {
    let mut widths = ColumnWidths {
        repo: 4,    // "REPO"
        branch: 6,  // "BRANCH"
        status: 6,  // "STATUS"
        stash: 5,   // "STASH"
        remote: 6,  // "REMOTE"
    };

    for info in repos {
        let path_len = display_path(&info.path, home_dir).len();
        widths.repo = widths.repo.max(path_len);
        widths.branch = widths.branch.max(info.branch_display().len());
        widths.status = widths.status.max(info.status_summary().len());
        widths.stash = widths.stash.max(info.stash_summary().len());
        widths.remote = widths.remote.max(info.remote_name.as_deref().unwrap_or("—").len());
    }

    widths
}

fn format_row(info: &RepoInfo, widths: &ColumnWidths, home_dir: Option<&std::path::Path>) -> String {
    let path_display = display_path(&info.path, home_dir);
    let remote_display = info.remote_name.as_deref().unwrap_or("—");

    format!(
        "{:<rw$}  {:<bw$}  {:<sw$}  {:<tw$}  {:<mw$}  {}",
        path_display,
        info.branch_display(),
        info.status_summary(),
        info.stash_summary(),
        remote_display,
        info.sync_summary(),
        rw = widths.repo, bw = widths.branch, sw = widths.status,
        tw = widths.stash, mw = widths.remote,
    )
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib static_output`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add src/static_output.rs
git commit -m "Add static columnar output with color-coded risk levels"
```

---

### Task 7: Wire Up Static Mode End-to-End

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Step 1: Update main.rs to use all modules**

```rust
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use grove::scanner::{self, ScanOptions};
use grove::git;
use grove::model::RepoInfo;
use grove::static_output;

#[derive(Parser, Debug)]
#[command(version, about = "Grove — would you lose work if this machine died?")]
struct Args {
    /// Directory to scan (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Static output mode (auto if not a TTY)
    #[arg(short = 'n', long = "no-interactive")]
    no_interactive: bool,

    /// Include hidden directories in traversal
    #[arg(short = 'H', long = "hidden")]
    hidden: bool,

    /// Maximum directory traversal depth
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Cross filesystem boundaries
    #[arg(long = "all-filesystems")]
    all_filesystems: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let interactive = !args.no_interactive && atty::is(atty::Stream::Stdout);
    let scan_path = args.path.canonicalize()?;
    let home_dir = dirs::home_dir();

    let opts = ScanOptions {
        include_hidden: args.hidden,
        max_depth: args.max_depth,
        cross_filesystems: args.all_filesystems,
    };

    // Scan for repos
    let repo_paths = scanner::scan_repos(&scan_path, &opts);

    // Inspect each repo
    let mut repos: Vec<RepoInfo> = repo_paths
        .iter()
        .filter_map(|p| match git::inspect_repo(p) {
            Ok(info) => Some(info),
            Err(e) => {
                eprintln!("Warning: failed to inspect {}: {}", p.display(), e);
                None
            }
        })
        .collect();

    // Sort by risk level (at-risk first)
    repos.sort_by_key(|r| r.risk_level());

    if interactive {
        // TUI mode — placeholder for now
        println!("Interactive mode not yet implemented. Use -n for static output.");
        static_output::print_static(&repos, home_dir.as_deref());
    } else {
        static_output::print_static(&repos, home_dir.as_deref());
    }

    Ok(())
}
```

Note: Add `dirs` crate for home directory detection.

**Step 2: Add dirs dependency**

Add to `Cargo.toml` under `[dependencies]`:
```toml
dirs = "6"
```

**Step 3: Verify end-to-end**

Run: `cargo run -- -n ~/git` (or wherever you have repos)
Expected: Columnar output showing repo statuses.

Run: `cargo run -- -n .` (from a directory without repos)
Expected: "No git repositories found."

**Step 4: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs
git commit -m "Wire up static mode: scan → inspect → classify → print"
```

---

### Task 8: TUI App State and Event Loop

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Implement TUI app state and event loop**

```rust
pub mod actions;
pub mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::model::RepoInfo;

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub detail_expanded: bool,
    pub should_quit: bool,
    pub scan_path: std::path::PathBuf,
    pub scan_options: crate::scanner::ScanOptions,
    pub home_dir: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(
        repos: Vec<RepoInfo>,
        scan_path: std::path::PathBuf,
        scan_options: crate::scanner::ScanOptions,
        home_dir: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            repos,
            selected: 0,
            detail_expanded: true,
            should_quit: false,
            scan_path,
            scan_options,
            home_dir,
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        self.repos.get(self.selected)
    }

    pub fn next(&mut self) {
        if !self.repos.is_empty() {
            self.selected = (self.selected + 1).min(self.repos.len() - 1);
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_detail(&mut self) {
        self.detail_expanded = !self.detail_expanded;
    }

    /// Refresh all repo statuses by re-scanning and re-inspecting.
    pub fn refresh_all(&mut self) {
        let repo_paths = crate::scanner::scan_repos(&self.scan_path, &self.scan_options);
        self.repos = repo_paths
            .iter()
            .filter_map(|p| crate::git::inspect_repo(p).ok())
            .collect();
        self.repos.sort_by_key(|r| r.risk_level());
        if self.selected >= self.repos.len() {
            self.selected = self.repos.len().saturating_sub(1);
        }
    }
}

pub fn run(app: &mut App) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(key, app, terminal)?;
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Enter => app.toggle_detail(),
        KeyCode::Char('r') => app.refresh_all(),
        KeyCode::Char('s') => actions::open_shell(app, terminal)?,
        KeyCode::Char('e') => actions::open_editor(app, terminal)?,
        KeyCode::Char('c') => actions::launch_claude(app, terminal, false)?,
        KeyCode::Char('C') => actions::launch_claude(app, terminal, true)?,
        KeyCode::Char('p') => actions::git_push(app)?,
        KeyCode::Char('f') => actions::git_fetch(app)?,
        KeyCode::Char('P') => actions::git_pull(app)?,
        KeyCode::Char('y') => actions::copy_path(app)?,
        _ => {}
    }
    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: May fail because `ui::draw` and actions don't exist yet — that's fine, we'll do those next.

**Step 3: Commit**

```bash
git add src/tui/mod.rs
git commit -m "Add TUI app state, event loop, and key handling"
```

---

### Task 9: TUI Rendering

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Implement the TUI layout and rendering**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::model::RiskLevel;
use super::App;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Header
            Constraint::Min(5),      // Repo list
            Constraint::Length(if app.detail_expanded { 12 } else { 0 }), // Detail panel
            Constraint::Length(1),   // Footer
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_repo_list(f, app, chunks[1]);
    if app.detail_expanded {
        draw_detail(f, app, chunks[2]);
    }
    draw_footer(f, app, chunks[3]);
}

fn risk_color(level: RiskLevel) -> Color {
    match level {
        RiskLevel::AtRisk => Color::Red,
        RiskLevel::Warning => Color::Yellow,
        RiskLevel::Safe => Color::Green,
    }
}

fn display_path(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home {
        if let Ok(stripped) = path.strip_prefix(home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let at_risk = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::AtRisk).count();
    let warning = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::Warning).count();
    let safe = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::Safe).count();

    let scan_path = display_path(&app.scan_path, app.home_dir.as_deref());

    let title = format!(" Grove — {} ", scan_path);
    let summary = format!("{} at risk, {} warning, {} safe", at_risk, warning, safe);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(Line::from(vec![
        Span::raw("  Repositories ("),
        Span::styled(format!("{at_risk} at risk"), Style::default().fg(Color::Red)),
        Span::raw(", "),
        Span::styled(format!("{warning} warning"), Style::default().fg(Color::Yellow)),
        Span::raw(", "),
        Span::styled(format!("{safe} safe"), Style::default().fg(Color::Green)),
        Span::raw(")"),
    ]))
    .block(block);

    f.render_widget(paragraph, area);
}

fn draw_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let color = risk_color(info.risk_level());
            let path_str = display_path(&info.path, app.home_dir.as_deref());
            let marker = if i == app.selected { "▸ " } else { "  " };

            let line = Line::from(vec![
                Span::raw(marker),
                Span::styled(
                    format!("{:<30}", path_str),
                    Style::default().fg(color),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{:<12}", info.branch_display()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:<16}", info.status_summary()),
                    Style::default().fg(color),
                ),
                Span::styled(
                    info.sync_summary(),
                    Style::default().fg(color),
                ),
            ]);

            let style = if i == app.selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(list, area);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(info) = app.selected_repo() else {
        return;
    };

    let path_str = display_path(&info.path, app.home_dir.as_deref());
    let title = format!(" {} ({}) ", path_str, info.branch_display());

    let mut lines: Vec<Line> = Vec::new();

    let dirty = info.modified_count + info.staged_count;
    if dirty > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Modified/staged files: {}", dirty),
            Style::default().fg(Color::Red),
        )));
    }
    if info.untracked_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Untracked files: {}", info.untracked_count),
            Style::default().fg(Color::Red),
        )));
    }
    if info.ahead > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Unpushed commits: {}", info.ahead),
            Style::default().fg(Color::Red),
        )));
    }
    if info.behind > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Behind remote: {}", info.behind),
            Style::default().fg(Color::Yellow),
        )));
    }
    if info.stash_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  Stashes: {}", info.stash_count),
            Style::default().fg(Color::Red),
        )));
    }
    if !info.has_remote {
        lines.push(Line::from(Span::styled(
            "  No remote configured",
            Style::default().fg(Color::Red),
        )));
    } else if !info.has_upstream && !info.is_detached {
        lines.push(Line::from(Span::styled(
            "  Branch has no upstream tracking",
            Style::default().fg(Color::Red),
        )));
    }
    if info.merge_in_progress {
        lines.push(Line::from(Span::styled(
            "  ⚠ Merge in progress",
            Style::default().fg(Color::Red),
        )));
    }
    if info.rebase_in_progress {
        lines.push(Line::from(Span::styled(
            "  ⚠ Rebase in progress",
            Style::default().fg(Color::Red),
        )));
    }
    if info.is_detached {
        lines.push(Line::from(Span::styled(
            "  Detached HEAD — not on any branch",
            Style::default().fg(Color::Yellow),
        )));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  ✓ All clean — fully synced with remote",
            Style::default().fg(Color::Green),
        )));
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut keys: Vec<Span> = vec![];

    // Context-sensitive keys based on selected repo
    if let Some(info) = app.selected_repo() {
        if info.ahead > 0 && info.has_remote {
            keys.push(Span::styled("[p]", Style::default().fg(Color::Cyan)));
            keys.push(Span::raw("ush  "));
        }
        if info.behind > 0 {
            keys.push(Span::styled("[P]", Style::default().fg(Color::Cyan)));
            keys.push(Span::raw("ull  "));
        }
        if info.has_remote {
            keys.push(Span::styled("[f]", Style::default().fg(Color::Cyan)));
            keys.push(Span::raw("etch  "));
        }
        if info.stash_count > 0 {
            keys.push(Span::styled("[t]", Style::default().fg(Color::Cyan)));
            keys.push(Span::raw("stash  "));
        }
    }

    keys.push(Span::styled("[s]", Style::default().fg(Color::Cyan)));
    keys.push(Span::raw("hell  "));
    keys.push(Span::styled("[e]", Style::default().fg(Color::Cyan)));
    keys.push(Span::raw("ditor  "));
    keys.push(Span::styled("[c]", Style::default().fg(Color::Cyan)));
    keys.push(Span::raw("laude  "));
    keys.push(Span::styled("[r]", Style::default().fg(Color::Cyan)));
    keys.push(Span::raw("efresh  "));
    keys.push(Span::styled("[q]", Style::default().fg(Color::Cyan)));
    keys.push(Span::raw("uit"));

    let footer = Paragraph::new(Line::from(keys));
    f.render_widget(footer, area);
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: May still fail if actions aren't stubbed — create stubs in next task, then circle back.

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "Add TUI rendering: header, repo list, detail panel, context-sensitive footer"
```

---

### Task 10: TUI Actions

**Files:**
- Modify: `src/tui/actions.rs`

**Step 1: Implement action handlers**

```rust
use std::io;
use std::process::Command;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::App;

/// Suspend TUI, run a command, then restore TUI.
fn suspend_and_run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut cmd: Command,
) -> Result<()> {
    // Restore terminal to normal mode
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Run the command
    let status = cmd.status();

    // Re-enter TUI mode
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    if let Err(e) = status {
        eprintln!("Command failed: {}", e);
    }

    Ok(())
}

pub fn open_shell(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let mut cmd = Command::new(&shell);
    cmd.current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn open_editor(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let mut cmd = Command::new(&editor);
    cmd.arg(".").current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn launch_claude(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    dangerously_skip_permissions: bool,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let mut cmd = Command::new("claude");
    if dangerously_skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    cmd.current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn git_push(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.ahead == 0 || !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["push"])
        .current_dir(&path)
        .output()?;

    // Refresh this repo's status
    if let Ok(updated) = crate::git::inspect_repo(&path) {
        if let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    }
    app.repos.sort_by_key(|r| r.risk_level());
    Ok(())
}

pub fn git_fetch(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["fetch"])
        .current_dir(&path)
        .output()?;

    if let Ok(updated) = crate::git::inspect_repo(&path) {
        if let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    }
    app.repos.sort_by_key(|r| r.risk_level());
    Ok(())
}

pub fn git_pull(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.behind == 0 { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["pull"])
        .current_dir(&path)
        .output()?;

    if let Ok(updated) = crate::git::inspect_repo(&path) {
        if let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    }
    app.repos.sort_by_key(|r| r.risk_level());
    Ok(())
}

pub fn copy_path(app: &App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let path_str = info.path.display().to_string();

    // macOS: pbcopy, Linux: xclip or xsel
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(path_str.as_bytes())?;
        }
        child.wait()?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        use std::io::Write;
        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(path_str.as_bytes())?;
            }
            child.wait()?;
        }
    }

    Ok(())
}
```

**Step 2: Update main.rs to launch TUI**

In `main.rs`, replace the interactive placeholder:

```rust
    if interactive {
        let scan_opts = ScanOptions {
            include_hidden: args.hidden,
            max_depth: args.max_depth,
            cross_filesystems: args.all_filesystems,
        };
        let mut app = grove::tui::App::new(repos, scan_path, scan_opts, home_dir);
        grove::tui::run(&mut app)?;
    } else {
        static_output::print_static(&repos, home_dir.as_deref());
    }
```

Note: `ScanOptions` needs to be constructed twice (once for scanning, once for TUI refresh). Either clone it, or derive Clone on it.

Add `#[derive(Clone)]` to `ScanOptions` in `scanner.rs`.

**Step 3: Verify full build**

Run: `cargo build`
Expected: Compiles successfully.

**Step 4: Manual test**

Run: `cargo run -- ~/git` (or a directory with repos)
Expected: Full-screen TUI with repo list, detail panel, navigation.

Run: `cargo run -- -n ~/git`
Expected: Static columnar output.

**Step 5: Commit**

```bash
git add -A
git commit -m "Add TUI actions and wire up interactive mode"
```

---

### Task 11: Polish and Edge Cases

**Files:**
- Various

**Step 1: Handle edge cases**

- Empty scan results: Show "No repositories found" in TUI mode too
- Very long paths: Truncate in TUI columns
- Permission errors during scan: Already handled (silent skip)
- git2 errors during inspection: Already handled (skip with warning)

**Step 2: Add progress indicator for scanning**

In `main.rs`, add a simple stderr message:

```rust
eprintln!("Scanning {}...", scan_path.display());
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Run clippy**

Run: `cargo clippy -- -W clippy::all`
Expected: No warnings (fix any that arise).

**Step 5: Final commit**

```bash
git add -A
git commit -m "Polish: edge cases, progress indicator, clippy fixes"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Project scaffolding | — |
| 2 | Data model (model.rs) | 12 unit tests |
| 3 | CLI argument parsing | Manual |
| 4 | Filesystem scanner | 7 unit tests |
| 5 | Git status inspection | 6 unit tests |
| 6 | Static output | 2 unit tests |
| 7 | Wire up static mode | Integration |
| 8 | TUI event loop | — |
| 9 | TUI rendering | — |
| 10 | TUI actions | Manual |
| 11 | Polish and edge cases | All |
