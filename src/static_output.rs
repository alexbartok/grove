use colored::Colorize;

use crate::model::{RepoInfo, RiskLevel};

struct ColumnWidths {
    repo: usize,
    branch: usize,
    status: usize,
    stash: usize,
    remote: usize,
}

/// Print repos as a colored columnar table to stdout.
pub fn print_static(repos: &[RepoInfo], home_dir: Option<&std::path::Path>) {
    if repos.is_empty() {
        println!("No git repositories found.");
        return;
    }

    let widths = compute_widths(repos, home_dir);

    // Summary
    let total = repos.len();
    let dirty = repos.iter().filter(|r| r.risk_level() != RiskLevel::Safe).count();
    if dirty > 0 {
        println!("{total} repos, {dirty} dirty\n");
    } else {
        println!("{total} repos, all clean\n");
    }

    // Header
    println!(
        "{:<rw$}  {:<bw$}  {:<sw$}  {:<tw$}  {:<mw$}  SYNC",
        "REPO",
        "BRANCH",
        "STATUS",
        "STASH",
        "REMOTE",
        rw = widths.repo,
        bw = widths.branch,
        sw = widths.status,
        tw = widths.stash,
        mw = widths.remote,
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

fn compute_widths(repos: &[RepoInfo], home_dir: Option<&std::path::Path>) -> ColumnWidths {
    let mut widths = ColumnWidths {
        repo: 4,    // "REPO"
        branch: 6,  // "BRANCH"
        status: 6,  // "STATUS"
        stash: 5,   // "STASH"
        remote: 6,  // "REMOTE"
    };

    for info in repos {
        widths.repo = widths.repo.max(crate::model::display_path(&info.path, home_dir).len());
        widths.branch = widths.branch.max(info.branch_display().len());
        widths.status = widths.status.max(info.status_summary().len());
        widths.stash = widths.stash.max(info.stash_summary().len());
        widths.remote = widths
            .remote
            .max(info.remote_name.as_deref().unwrap_or("\u{2014}").len());
    }

    widths
}

fn format_row(
    info: &RepoInfo,
    widths: &ColumnWidths,
    home_dir: Option<&std::path::Path>,
) -> String {
    let path_display = crate::model::display_path(&info.path, home_dir);
    let remote_display = info.remote_name.as_deref().unwrap_or("\u{2014}");

    format!(
        "{:<rw$}  {:<bw$}  {:<sw$}  {:<tw$}  {:<mw$}  {}",
        path_display,
        info.branch_display(),
        info.status_summary(),
        info.stash_summary(),
        remote_display,
        info.sync_summary(),
        rw = widths.repo,
        bw = widths.branch,
        sw = widths.status,
        tw = widths.stash,
        mw = widths.remote,
    )
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
        }
    }

    #[test]
    fn test_tilde_contraction() {
        let home = PathBuf::from("/Users/alice");
        let result = crate::model::display_path(&PathBuf::from("/Users/alice/projects/foo"), Some(&home));
        assert_eq!(result, "~/projects/foo");
    }

    #[test]
    fn test_no_tilde_when_not_under_home() {
        let home = PathBuf::from("/Users/alice");
        let result = crate::model::display_path(&PathBuf::from("/tmp/foo"), Some(&home));
        assert_eq!(result, "/tmp/foo");
    }

    #[test]
    fn test_format_row_contains_all_columns() {
        let info = make_safe_repo("/tmp/repo");
        let widths = ColumnWidths {
            repo: 20,
            branch: 10,
            status: 10,
            stash: 5,
            remote: 10,
        };
        let row = format_row(&info, &widths, None);
        assert!(row.contains("main"));
        assert!(row.contains("✓ clean"));
        assert!(row.contains("origin"));
        assert!(row.contains("✓ clean"));
    }

    #[test]
    fn test_format_row_with_tilde() {
        let info = make_safe_repo("/Users/alice/projects/foo");
        let home = PathBuf::from("/Users/alice");
        let widths = ColumnWidths {
            repo: 30,
            branch: 10,
            status: 10,
            stash: 5,
            remote: 10,
        };
        let row = format_row(&info, &widths, Some(&home));
        assert!(row.contains("~/projects/foo"));
    }
}
