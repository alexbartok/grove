use std::path::PathBuf;

use crate::config::Config;

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
    pub remote_host: Option<String>,
    pub remote_urls: Vec<(String, String)>,
    pub remote_count: usize,
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

    /// Display the upstream host/service name, with optional "+N" for extra remotes.
    pub fn host_display(&self, config: &Config) -> String {
        match &self.remote_host {
            None => "\u{2014}".to_string(),
            Some(hostname) => {
                let label = if let Some(alias) = config.host_aliases.get(hostname.as_str()) {
                    alias.clone()
                } else if let Some(builtin) = builtin_host_name(hostname) {
                    builtin.to_string()
                } else {
                    hostname.clone()
                };
                if self.remote_count > 1 {
                    format!("{} +{}", label, self.remote_count - 1)
                } else {
                    label
                }
            }
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
            (false, false) => String::new(),
        }
    }
}

/// Map well-known hostnames to short display names.
fn builtin_host_name(hostname: &str) -> Option<&'static str> {
    match hostname {
        "github.com" => Some("GitHub"),
        "gitlab.com" => Some("GitLab"),
        "bitbucket.org" => Some("Bitbucket"),
        "codeberg.org" => Some("Codeberg"),
        "sr.ht" | "git.sr.ht" => Some("SourceHut"),
        _ => None,
    }
}

/// Extract the hostname from a git remote URL.
///
/// Handles HTTPS (`https://github.com/user/repo.git`),
/// SCP-style SSH (`git@github.com:user/repo.git`),
/// and SSH with scheme (`ssh://git@host/repo`).
/// Returns `None` for local paths.
pub fn parse_host_from_url(url: &str) -> Option<String> {
    // HTTPS / SSH with scheme: starts with a scheme
    if let Some(rest) = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ssh://"))
        .or_else(|| url.strip_prefix("git://"))
    {
        // Strip user@ prefix if present
        let after_user = rest.split_once('@').map(|(_, h)| h).unwrap_or(rest);
        // Take host part before / or : (also strips port numbers)
        let host = after_user.split(&['/', ':'][..]).next()?;
        if host.is_empty() {
            return None;
        }
        return Some(host.to_lowercase());
    }

    // SCP-style: user@host:path (no scheme, must contain @ and :)
    if let Some(at_pos) = url.find('@') {
        let after_at = &url[at_pos + 1..];
        if let Some(colon_pos) = after_at.find(':') {
            let host = &after_at[..colon_pos];
            if !host.is_empty() && !host.contains('/') {
                return Some(host.to_lowercase());
            }
        }
    }

    // Local path or unrecognized format
    None
}

/// Shorten a path by replacing $HOME prefix with `~`.
pub fn display_path(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home
        && let Ok(stripped) = path.strip_prefix(home)
    {
        return format!("~/{}", stripped.display());
    }
    path.display().to_string()
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
            remote_host: Some("github.com".to_string()),
            remote_urls: vec![("origin".to_string(), "https://github.com/user/repo.git".to_string())],
            remote_count: 1,
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
        assert_eq!(repo.sync_summary(), "");
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
        // detached heads without upstream should show empty, not "no tracking"
        assert_eq!(repo.sync_summary(), "");
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

    // --- URL parsing tests ---

    #[test]
    fn test_parse_host_https() {
        assert_eq!(parse_host_from_url("https://github.com/user/repo.git"), Some("github.com".into()));
        assert_eq!(parse_host_from_url("https://gitlab.com/user/repo"), Some("gitlab.com".into()));
    }

    #[test]
    fn test_parse_host_ssh_scp() {
        assert_eq!(parse_host_from_url("git@github.com:user/repo.git"), Some("github.com".into()));
        assert_eq!(parse_host_from_url("git@git.iguana-galaxy.ts.net:user/repo.git"), Some("git.iguana-galaxy.ts.net".into()));
    }

    #[test]
    fn test_parse_host_ssh_scheme() {
        assert_eq!(parse_host_from_url("ssh://git@github.com/user/repo.git"), Some("github.com".into()));
    }

    #[test]
    fn test_parse_host_git_scheme() {
        assert_eq!(parse_host_from_url("git://example.com/repo.git"), Some("example.com".into()));
    }

    #[test]
    fn test_parse_host_local_path() {
        assert_eq!(parse_host_from_url("/path/to/repo.git"), None);
        assert_eq!(parse_host_from_url("../relative/repo"), None);
    }

    #[test]
    fn test_parse_host_case_insensitive() {
        assert_eq!(parse_host_from_url("https://GitHub.COM/user/repo.git"), Some("github.com".into()));
    }

    // --- Host display tests ---

    #[test]
    fn test_host_display_builtin() {
        let repo = default_repo();
        let config = Config::default();
        assert_eq!(repo.host_display(&config), "GitHub");
    }

    #[test]
    fn test_host_display_config_override() {
        let repo = default_repo();
        let mut config = Config::default();
        config.host_aliases.insert("github.com".into(), "GH".into());
        assert_eq!(repo.host_display(&config), "GH");
    }

    #[test]
    fn test_host_display_unknown_host() {
        let mut repo = default_repo();
        repo.remote_host = Some("git.iguana-galaxy.ts.net".into());
        let config = Config::default();
        assert_eq!(repo.host_display(&config), "git.iguana-galaxy.ts.net");
    }

    #[test]
    fn test_host_display_multi_remote() {
        let mut repo = default_repo();
        repo.remote_count = 3;
        let config = Config::default();
        assert_eq!(repo.host_display(&config), "GitHub +2");
    }

    #[test]
    fn test_host_display_no_remote() {
        let mut repo = default_repo();
        repo.has_remote = false;
        repo.remote_host = None;
        repo.remote_count = 0;
        let config = Config::default();
        assert_eq!(repo.host_display(&config), "\u{2014}");
    }
}
