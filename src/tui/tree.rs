use std::collections::BTreeMap;
use std::path::Path;

use crate::model::RepoInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Tree,
    Dirty,
}

#[derive(Debug, Clone)]
pub enum DisplayRow {
    Directory {
        name: String,
        tree_prefix: String,
    },
    Repo {
        repo_index: usize,
        display_name: String,
        tree_prefix: String,
    },
}

impl DisplayRow {
    pub fn display_name(&self) -> &str {
        match self {
            DisplayRow::Directory { name, .. } => name,
            DisplayRow::Repo { display_name, .. } => display_name,
        }
    }

    pub fn tree_prefix(&self) -> &str {
        match self {
            DisplayRow::Directory { tree_prefix, .. } | DisplayRow::Repo { tree_prefix, .. } => {
                tree_prefix
            }
        }
    }

    pub fn repo_index(&self) -> Option<usize> {
        match self {
            DisplayRow::Directory { .. } => None,
            DisplayRow::Repo { repo_index, .. } => Some(*repo_index),
        }
    }
}

struct TrieNode {
    repo_index: Option<usize>,
    children: BTreeMap<String, TrieNode>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            repo_index: None,
            children: BTreeMap::new(),
        }
    }
}

/// Build display rows as an alphabetical tree view.
pub fn build_tree_rows(repos: &[RepoInfo], scan_path: &Path) -> Vec<DisplayRow> {
    let mut root = TrieNode::new();

    for (idx, repo) in repos.iter().enumerate() {
        let rel = repo.path.strip_prefix(scan_path).unwrap_or(&repo.path);
        let components: Vec<String> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(String::from))
            .collect();

        let mut node = &mut root;
        for component in &components {
            node = node
                .children
                .entry(component.clone())
                .or_insert_with(TrieNode::new);
        }
        node.repo_index = Some(idx);
    }

    let mut rows = Vec::new();
    flatten_children(&root, &mut rows, String::new());
    rows
}

fn flatten_children(node: &TrieNode, rows: &mut Vec<DisplayRow>, prefix: String) {
    let children: Vec<(&String, &TrieNode)> = node.children.iter().collect();
    let count = children.len();

    for (i, (name, child)) in children.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let continuation = if is_last { "    " } else { "│   " };

        let tree_prefix = format!("{prefix}{connector}");
        let child_prefix = format!("{prefix}{continuation}");

        if let Some(repo_index) = child.repo_index {
            rows.push(DisplayRow::Repo {
                repo_index,
                display_name: name.to_string(),
                tree_prefix,
            });
            if !child.children.is_empty() {
                flatten_children(child, rows, child_prefix);
            }
        } else {
            rows.push(DisplayRow::Directory {
                name: name.to_string(),
                tree_prefix,
            });
            flatten_children(child, rows, child_prefix);
        }
    }
}

/// Build display rows as a flat list (dirty-first sort mode).
pub fn build_flat_rows(repos: &[RepoInfo], home_dir: Option<&Path>) -> Vec<DisplayRow> {
    repos
        .iter()
        .enumerate()
        .map(|(idx, repo)| DisplayRow::Repo {
            repo_index: idx,
            display_name: crate::model::display_path(&repo.path, home_dir),
            tree_prefix: String::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_repo(path: &str) -> RepoInfo {
        RepoInfo {
            path: PathBuf::from(path),
            branch: Some("main".into()),
            is_detached: false,
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            has_remote: true,
            remote_host: Some("github.com".into()),
            remote_urls: vec![("origin".into(), "https://github.com/user/repo.git".into())],
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
    fn tree_flat_siblings() {
        let repos = vec![
            make_repo("/scan/alpha"),
            make_repo("/scan/beta"),
            make_repo("/scan/gamma"),
        ];
        let rows = build_tree_rows(&repos, Path::new("/scan"));

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].display_name(), "alpha");
        assert_eq!(rows[1].display_name(), "beta");
        assert_eq!(rows[2].display_name(), "gamma");
        assert!(rows[0].tree_prefix().contains('├'));
        assert!(rows[2].tree_prefix().contains('└'));
    }

    #[test]
    fn tree_nested_directory() {
        let repos = vec![
            make_repo("/scan/dir/repo-a"),
            make_repo("/scan/dir/repo-b"),
            make_repo("/scan/solo"),
        ];
        let rows = build_tree_rows(&repos, Path::new("/scan"));

        assert_eq!(rows.len(), 4);
        assert!(matches!(&rows[0], DisplayRow::Directory { name, .. } if name == "dir"));
        assert_eq!(rows[1].display_name(), "repo-a");
        assert_eq!(rows[2].display_name(), "repo-b");
        assert_eq!(rows[3].display_name(), "solo");
    }

    #[test]
    fn tree_repo_with_children() {
        let repos = vec![
            make_repo("/scan/parent"),
            make_repo("/scan/parent/child"),
        ];
        let rows = build_tree_rows(&repos, Path::new("/scan"));

        assert_eq!(rows.len(), 2);
        assert!(
            matches!(&rows[0], DisplayRow::Repo { display_name, .. } if display_name == "parent")
        );
        assert!(
            matches!(&rows[1], DisplayRow::Repo { display_name, .. } if display_name == "child")
        );
    }

    #[test]
    fn tree_alphabetical_order() {
        let repos = vec![
            make_repo("/scan/zebra"),
            make_repo("/scan/alpha"),
            make_repo("/scan/middle"),
        ];
        let rows = build_tree_rows(&repos, Path::new("/scan"));

        assert_eq!(rows[0].display_name(), "alpha");
        assert_eq!(rows[0].repo_index(), Some(1));
        assert_eq!(rows[1].display_name(), "middle");
        assert_eq!(rows[1].repo_index(), Some(2));
        assert_eq!(rows[2].display_name(), "zebra");
        assert_eq!(rows[2].repo_index(), Some(0));
    }

    #[test]
    fn flat_rows_with_home_contraction() {
        let repos = vec![make_repo("/home/user/projects/foo")];
        let home = PathBuf::from("/home/user");
        let rows = build_flat_rows(&repos, Some(&home));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].display_name(), "~/projects/foo");
        assert_eq!(rows[0].tree_prefix(), "");
    }

    #[test]
    fn tree_deep_nesting() {
        let repos = vec![
            make_repo("/scan/a/b/c"),
            make_repo("/scan/a/b/d"),
        ];
        let rows = build_tree_rows(&repos, Path::new("/scan"));

        assert_eq!(rows.len(), 4);
        assert!(matches!(&rows[0], DisplayRow::Directory { name, .. } if name == "a"));
        assert!(matches!(&rows[1], DisplayRow::Directory { name, .. } if name == "b"));
        assert_eq!(rows[2].display_name(), "c");
        assert_eq!(rows[3].display_name(), "d");
        assert!(rows[2].tree_prefix().contains('├'));
        assert!(rows[3].tree_prefix().contains('└'));
    }
}
