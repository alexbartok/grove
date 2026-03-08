use std::path::PathBuf;

/// Risk classification for a repository's state.
/// Ordered so that `AtRisk` sorts first (lowest ordinal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    AtRisk,
    Warning,
    Safe,
}

/// Collected status information about a single Git repository.
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
    /// Classify the repository into a risk level.
    ///
    /// `AtRisk` if any of: uncommitted changes (modified, staged, untracked),
    /// no remote, unpushed commits, stashes present, no upstream tracking
    /// branch (unless detached), merge or rebase in progress.
    ///
    /// `Warning` if detached HEAD (and none of the AtRisk conditions apply).
    ///
    /// `Safe` otherwise.
    pub fn risk_level(&self) -> RiskLevel {
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
            RiskLevel::AtRisk
        } else if self.is_detached {
            RiskLevel::Warning
        } else {
            RiskLevel::Safe
        }
    }

    /// Human-readable branch name: "HEAD" when detached, the branch name
    /// when available, or "???" as a last resort.
    pub fn branch_display(&self) -> &str {
        if self.is_detached {
            "HEAD"
        } else {
            self.branch.as_deref().unwrap_or("???")
        }
    }

    /// One-line working-tree status summary.
    pub fn status_summary(&self) -> String {
        if self.merge_in_progress {
            return "\u{26a0} merging".to_string();
        }
        if self.rebase_in_progress {
            return "\u{26a0} rebasing".to_string();
        }

        let dirty = self.modified_count + self.staged_count;

        match (dirty > 0, self.untracked_count > 0) {
            (true, true) => format!("{} modified, {} untracked", dirty, self.untracked_count),
            (true, false) => format!("{dirty} modified"),
            (false, true) => format!("{} untracked", self.untracked_count),
            (false, false) => "\u{2713} clean".to_string(),
        }
    }

    /// Stash count as a string, or an em-dash when there are none.
    pub fn stash_summary(&self) -> String {
        if self.stash_count > 0 {
            self.stash_count.to_string()
        } else {
            "\u{2014}".to_string()
        }
    }

    /// One-line remote/sync status summary.
    pub fn sync_summary(&self) -> String {
        if !self.has_remote {
            return "\u{2717} no remote".to_string();
        }
        if !self.has_upstream && !self.is_detached {
            return "\u{26a0} no tracking".to_string();
        }

        match (self.ahead > 0, self.behind > 0) {
            (true, true) => format!("\u{2191}{} \u{2193}{}", self.ahead, self.behind),
            (true, false) => format!("\u{2191}{} ahead", self.ahead),
            (false, true) => format!("\u{2193}{} behind", self.behind),
            (false, false) => "\u{2713} synced".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_repo() -> RepoInfo {
        RepoInfo {
            path: PathBuf::from("/tmp/repo"),
            branch: Some("main".to_string()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_name: Some("origin".to_string()),
            has_upstream: true,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            merge_in_progress: false,
            rebase_in_progress: false,
        }
    }

    #[test]
    fn test_risk_at_risk_uncommitted_changes() {
        let mut repo = default_repo();
        repo.modified_count = 3;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);

        let mut repo2 = default_repo();
        repo2.staged_count = 2;
        assert_eq!(repo2.risk_level(), RiskLevel::AtRisk);

        let mut repo3 = default_repo();
        repo3.untracked_count = 1;
        assert_eq!(repo3.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_no_remote() {
        let mut repo = default_repo();
        repo.has_remote = false;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_unpushed() {
        let mut repo = default_repo();
        repo.ahead = 5;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_stashes() {
        let mut repo = default_repo();
        repo.stash_count = 2;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_no_upstream() {
        let mut repo = default_repo();
        repo.has_upstream = false;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_merge_in_progress() {
        let mut repo = default_repo();
        repo.merge_in_progress = true;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_at_risk_rebase_in_progress() {
        let mut repo = default_repo();
        repo.rebase_in_progress = true;
        assert_eq!(repo.risk_level(), RiskLevel::AtRisk);
    }

    #[test]
    fn test_risk_warning_detached_head() {
        let mut repo = default_repo();
        repo.is_detached = true;
        // detached head means no upstream, but is_detached exempts it from AtRisk for that reason
        // however has_upstream=true here so only detached triggers warning
        assert_eq!(repo.risk_level(), RiskLevel::Warning);
    }

    #[test]
    fn test_risk_safe() {
        let repo = default_repo();
        assert_eq!(repo.risk_level(), RiskLevel::Safe);
    }

    #[test]
    fn test_sorting_by_risk() {
        let mut safe = default_repo();
        safe.path = PathBuf::from("/tmp/safe");

        let mut warning = default_repo();
        warning.path = PathBuf::from("/tmp/warning");
        warning.is_detached = true;

        let mut at_risk = default_repo();
        at_risk.path = PathBuf::from("/tmp/at_risk");
        at_risk.modified_count = 1;

        let mut repos = vec![safe, warning, at_risk];
        repos.sort_by_key(|r| r.risk_level());

        assert_eq!(repos[0].risk_level(), RiskLevel::AtRisk);
        assert_eq!(repos[1].risk_level(), RiskLevel::Warning);
        assert_eq!(repos[2].risk_level(), RiskLevel::Safe);
    }

    #[test]
    fn test_branch_display_normal() {
        let repo = default_repo();
        assert_eq!(repo.branch_display(), "main");
    }

    #[test]
    fn test_branch_display_detached() {
        let mut repo = default_repo();
        repo.is_detached = true;
        assert_eq!(repo.branch_display(), "HEAD");
    }

    #[test]
    fn test_branch_display_fallback() {
        let mut repo = default_repo();
        repo.branch = None;
        assert_eq!(repo.branch_display(), "???");
    }

    #[test]
    fn test_status_summary_clean() {
        let repo = default_repo();
        assert_eq!(repo.status_summary(), "✓ clean");
    }

    #[test]
    fn test_status_summary_modified() {
        let mut repo = default_repo();
        repo.modified_count = 2;
        repo.staged_count = 1;
        assert_eq!(repo.status_summary(), "3 modified");
    }

    #[test]
    fn test_status_summary_untracked_only() {
        let mut repo = default_repo();
        repo.untracked_count = 4;
        assert_eq!(repo.status_summary(), "4 untracked");
    }

    #[test]
    fn test_status_summary_modified_and_untracked() {
        let mut repo = default_repo();
        repo.modified_count = 2;
        repo.untracked_count = 3;
        assert_eq!(repo.status_summary(), "2 modified, 3 untracked");
    }

    #[test]
    fn test_status_summary_merging() {
        let mut repo = default_repo();
        repo.merge_in_progress = true;
        assert_eq!(repo.status_summary(), "⚠ merging");
    }

    #[test]
    fn test_status_summary_rebasing() {
        let mut repo = default_repo();
        repo.rebase_in_progress = true;
        assert_eq!(repo.status_summary(), "⚠ rebasing");
    }

    #[test]
    fn test_stash_summary_with_stashes() {
        let mut repo = default_repo();
        repo.stash_count = 3;
        assert_eq!(repo.stash_summary(), "3");
    }

    #[test]
    fn test_stash_summary_no_stashes() {
        let repo = default_repo();
        assert_eq!(repo.stash_summary(), "\u{2014}");
    }

    #[test]
    fn test_sync_summary_synced() {
        let repo = default_repo();
        assert_eq!(repo.sync_summary(), "✓ synced");
    }

    #[test]
    fn test_sync_summary_no_remote() {
        let mut repo = default_repo();
        repo.has_remote = false;
        assert_eq!(repo.sync_summary(), "✗ no remote");
    }

    #[test]
    fn test_sync_summary_no_tracking() {
        let mut repo = default_repo();
        repo.has_upstream = false;
        assert_eq!(repo.sync_summary(), "⚠ no tracking");
    }

    #[test]
    fn test_sync_summary_no_tracking_detached_exemption() {
        let mut repo = default_repo();
        repo.has_upstream = false;
        repo.is_detached = true;
        // detached heads without upstream should show synced, not "no tracking"
        assert_eq!(repo.sync_summary(), "✓ synced");
    }

    #[test]
    fn test_sync_summary_ahead() {
        let mut repo = default_repo();
        repo.ahead = 3;
        assert_eq!(repo.sync_summary(), "↑3 ahead");
    }

    #[test]
    fn test_sync_summary_behind() {
        let mut repo = default_repo();
        repo.behind = 2;
        assert_eq!(repo.sync_summary(), "↓2 behind");
    }

    #[test]
    fn test_sync_summary_ahead_and_behind() {
        let mut repo = default_repo();
        repo.ahead = 3;
        repo.behind = 2;
        assert_eq!(repo.sync_summary(), "↑3 ↓2");
    }
}
