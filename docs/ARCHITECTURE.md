# Architecture

## Data flow

```
main.rs          CLI args, mode dispatch
   │
   ▼
scanner.rs       Walk filesystem, yield repo paths
   │
   ▼
git.rs           Inspect each repo path → RepoInfo
   │
   ▼
model.rs         Classify risk level, sort repos
   │
   ├──────────────────────┐
   ▼                      ▼
static_output.rs      tui/
Print and exit        Interactive app
                         ├── mod.rs     App state, event loop
                         ├── ui.rs      Layout, rendering
                         └── actions.rs Action handlers
```

## Modules

### `model.rs`

Defines the core types shared by all other modules.

- **`RiskLevel`** — `AtRisk`, `Warning`, `Safe`. Derives `Ord` so `AtRisk` sorts first. Used for sorting and color selection, but the labels are not shown to users.
- **`RepoInfo`** — Complete status snapshot of a git repo. All fields are populated by `git.rs` and consumed by the output modules.
- Display helper methods: `risk_level()`, `branch_display()`, `status_summary()`, `stash_summary()`, `sync_summary()`.

### `scanner.rs`

Filesystem traversal. Returns `Vec<PathBuf>` of directories containing `.git`.

- Recursive walk with `std::fs::read_dir`
- Does not descend into repos (stops at `.git`)
- `ScanOptions` controls: hidden directory inclusion, max depth, filesystem boundary crossing
- Filesystem boundary detection via `MetadataExt::dev()` (Unix)
- Silently skips unreadable directories

### `git.rs`

Uses `git2` (libgit2 bindings) to inspect a single repo and produce a `RepoInfo`.

Helper functions:
- `get_branch_info` — branch name, detached HEAD, handles unborn branches
- `get_working_tree_status` — counts modified, staged, untracked files via `repo.statuses()`
- `get_remote_info` — whether remotes exist, first remote name
- `get_upstream_info` — upstream tracking, ahead/behind via `graph_ahead_behind`
- `get_stash_count` — iterates stash entries via `stash_foreach`
- Merge/rebase detection via `.git/MERGE_HEAD`, `.git/rebase-merge`, `.git/rebase-apply`

### `static_output.rs`

Non-interactive columnar output. Auto-computes column widths from data. Rows colored by risk level. Paths contracted with `~/` when under home directory.

### `tui/mod.rs`

TUI application state and event loop.

- **`App`** — holds repos, selection state, `ListState` for scrolling, scan config
- **Event loop** — 100ms poll interval, crossterm key events
- **Key handler** — dispatches to navigation methods or `actions.rs` functions
- Terminal setup/teardown with alternate screen and raw mode

### `tui/ui.rs`

Rendering with ratatui. Four vertical sections:
1. **Header** — title + repo count summary
2. **Repo list** — scrollable list with dynamic column widths, scrollbar
3. **Detail panel** — status indicators for selected repo (collapsible)
4. **Footer** — context-sensitive keybinding hints

### `tui/actions.rs`

Action handlers invoked from key bindings.

- **Suspend/resume** pattern: `open_shell`, `open_editor`, `launch_claude` leave alternate screen, run the command, then restore the TUI
- **In-place git ops**: `git_push`, `git_fetch`, `git_pull` run the command, re-inspect the repo, re-sort
- **Clipboard**: `copy_path` uses `pbcopy` (macOS) or `xclip` (Linux)

## Design decisions

**git2 over shelling out** — native bindings are faster, don't require git installed, and avoid parsing command output.

**Scan-then-display** — all repos are discovered and inspected before rendering. Simpler than streaming, acceptable for typical directory trees.

**No config file** — all behavior via CLI flags. Keeps the tool predictable and stateless.

**Suspend/resume for external commands** — shell, editor, and claude cleanly leave the TUI rather than embedding a terminal. Simpler and more reliable.

**Risk model internal only** — `RiskLevel` drives sorting and colors but the labels "at risk"/"warning"/"safe" are not shown in the UI. The color and specific indicators (e.g. "2 modified", "no remote") communicate status without categorization.

**stdlib over crates for basics** — `std::io::IsTerminal` instead of `atty`, `$HOME` env var instead of `dirs` crate. Fewer dependencies for trivial functionality.
