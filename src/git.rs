use std::path::Path;

use anyhow::Result;
use git2::{Repository, Status, StatusOptions};

use crate::model::RepoInfo;

/// Inspect a git repository and return its full status.
pub fn inspect_repo(path: &Path) -> Result<RepoInfo> {
    let mut repo = Repository::open(path)?;

    let (branch, is_detached) = get_branch_info(&repo);
    let (modified_count, staged_count, untracked_count) = get_working_tree_status(&repo);
    let (has_remote, remote_name) = get_remote_info(&repo);
    let (has_upstream, ahead, behind) = get_upstream_info(&repo);
    let stash_count = get_stash_count(&mut repo);
    let merge_in_progress = repo.path().join("MERGE_HEAD").exists();
    let rebase_in_progress =
        repo.path().join("rebase-merge").exists() || repo.path().join("rebase-apply").exists();

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

/// Determine the current branch name and whether HEAD is detached.
///
/// For an unborn branch (empty repo with no commits), we read the symbolic
/// target of HEAD and strip the `refs/heads/` prefix.
fn get_branch_info(repo: &Repository) -> (Option<String>, bool) {
    let is_detached = repo.head_detached().unwrap_or(false);

    match repo.head() {
        Ok(head) => {
            let branch = head.shorthand().map(|s| s.to_string());
            (branch, is_detached)
        }
        Err(e) => {
            // If HEAD is an unborn branch (empty repo), git2 returns NotFound.
            // We can still determine the branch name from the symbolic target.
            if e.code() == git2::ErrorCode::UnbornBranch {
                let branch = repo
                    .find_reference("HEAD")
                    .ok()
                    .and_then(|r| r.symbolic_target().map(|s| s.to_string()))
                    .map(|s| s.strip_prefix("refs/heads/").unwrap_or(&s).to_string());
                (branch, false)
            } else {
                (None, is_detached)
            }
        }
    }
}

/// Count modified, staged, and untracked files in the working tree.
fn get_working_tree_status(repo: &Repository) -> (usize, usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (0, 0, 0),
    };

    let mut modified = 0usize;
    let mut staged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();

        // Working tree modifications
        if status.intersects(
            Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_RENAMED
                | Status::WT_TYPECHANGE,
        ) {
            modified += 1;
        }

        // Index (staged) changes
        if status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            staged += 1;
        }

        // Untracked files
        if status.contains(Status::WT_NEW) {
            untracked += 1;
        }
    }

    (modified, staged, untracked)
}

/// Check whether any remotes exist and return the name of the first one.
fn get_remote_info(repo: &Repository) -> (bool, Option<String>) {
    match repo.remotes() {
        Ok(remotes) => {
            if remotes.is_empty() {
                (false, None)
            } else {
                let name = remotes.get(0).map(|s| s.to_string());
                (true, name)
            }
        }
        Err(_) => (false, None),
    }
}

/// Determine whether the current branch has an upstream tracking branch,
/// and compute ahead/behind counts.
fn get_upstream_info(repo: &Repository) -> (bool, usize, usize) {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return (false, 0, 0),
    };

    // Only meaningful for non-detached HEAD pointing at a branch
    if repo.head_detached().unwrap_or(false) {
        return (false, 0, 0);
    }

    let branch_name = match head.shorthand() {
        Some(name) => name.to_string(),
        None => return (false, 0, 0),
    };

    let local_branch = match repo.find_branch(&branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return (false, 0, 0),
    };

    let upstream = match local_branch.upstream() {
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

/// Count the number of stash entries.
fn get_stash_count(repo: &mut Repository) -> usize {
    let mut count = 0usize;
    // stash_foreach returns an error if the callback ever returns false,
    // but we always return true, so any error here means no stashes (or
    // the stash ref doesn't exist).
    let _ = repo.stash_foreach(|_, _, _| {
        count += 1;
        true
    });
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
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
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    #[test]
    fn test_clean_repo_no_remote() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.branch.is_some());
        assert!(!info.is_detached);
        assert_eq!(info.modified_count, 0);
        assert_eq!(info.untracked_count, 0);
        assert!(!info.has_remote);
    }

    #[test]
    fn test_modified_files() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());

        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }
        make_initial_commit(&repo);

        // Modify the file after committing
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
        let mut repo = init_repo(tmp.path());

        fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
        }
        make_initial_commit(&repo);

        // Create a modification and stash it
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

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.branch.is_some());
        assert!(!info.has_remote);
    }

    #[test]
    fn test_detached_head() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        // Detach HEAD by checking out the commit directly
        let head_oid = repo.head().unwrap().target().unwrap();
        repo.set_head_detached(head_oid).unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.is_detached);
    }

    #[test]
    fn test_merge_in_progress() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        // Simulate merge in progress by creating MERGE_HEAD
        let git_dir = repo.path();
        fs::write(git_dir.join("MERGE_HEAD"), "dummy").unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.merge_in_progress);
    }

    #[test]
    fn test_rebase_in_progress() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        // Simulate rebase in progress by creating rebase-merge directory
        let git_dir = repo.path();
        fs::create_dir(git_dir.join("rebase-merge")).unwrap();

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(info.rebase_in_progress);
    }

    #[test]
    fn test_no_merge_or_rebase() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path());
        make_initial_commit(&repo);

        let info = inspect_repo(tmp.path()).unwrap();
        assert!(!info.merge_in_progress);
        assert!(!info.rebase_in_progress);
    }
}
