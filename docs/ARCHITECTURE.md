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

All repos are found and inspected before anything is displayed.

## Modules

### model.rs

Types used everywhere else.

- `RiskLevel` — `AtRisk`, `Warning`, `Safe`. Derives `Ord` so `AtRisk` sorts first. Used internally for sorting and color; the labels themselves don't appear in the UI.
- `RepoInfo` — everything we know about a repo. Populated by `git.rs`, read by the output modules.
- Helper methods on `RepoInfo`: `risk_level()`, `branch_display()`, `status_summary()`, `stash_summary()`, `sync_summary()`.

### scanner.rs

Walks directories with `std::fs::read_dir`, returns `Vec<PathBuf>` of directories containing `.git`.

- Stops descending when it hits a `.git` directory
- `ScanOptions` controls hidden directory inclusion, max depth, and filesystem boundary crossing
- Boundary detection uses `MetadataExt::dev()` on Unix
- Skips directories it can't read

### git.rs

Opens each repo with `git2` and fills in a `RepoInfo`.

- `get_branch_info` — branch name, detached HEAD, unborn branches
- `get_working_tree_status` — modified/staged/untracked counts from `repo.statuses()`
- `get_remote_info` — whether remotes exist, first remote name
- `get_upstream_info` — upstream tracking, ahead/behind from `graph_ahead_behind`
- `get_stash_count` — iterates stash entries with `stash_foreach`
- Merge/rebase detection checks for `.git/MERGE_HEAD`, `.git/rebase-merge`, `.git/rebase-apply`

### static_output.rs

Columnar output for non-interactive mode. Computes column widths from the data, colors rows by risk level, contracts paths under `$HOME` to `~/`.

### tui/mod.rs

App state and event loop.

- `App` holds the repo list, selection state, `ListState` for scrolling, and scan config
- Event loop polls crossterm every 100ms
- Key handler dispatches to navigation methods or action functions

### tui/ui.rs

Rendering with ratatui. The screen has four vertical sections: header (title + summary), repo list (scrollable, with scrollbar), detail panel (collapsible), and footer (keybinding hints that change based on the selected repo).

### tui/actions.rs

Handles what happens when you press an action key.

- Shell, editor, and claude: leave the alternate screen, run the command, restore the TUI when it exits
- Git push/fetch/pull: run the command, re-inspect the affected repo, re-sort the list
- Copy path: pipes to `pbcopy` on macOS, `xclip` on Linux

## Why these choices

**git2 instead of shelling out to git** — faster, doesn't require git to be installed, no output parsing.

**Scan everything, then display** — simpler than streaming. Fine for typical project directories.

**No config file** — CLI flags only. Nothing to discover or debug.

**Suspend/resume for external commands** — leaving the TUI to run a shell or editor is simpler and more reliable than trying to embed one.

**stdlib where possible** — `std::io::IsTerminal` instead of the `atty` crate, `$HOME` instead of `dirs`. Less to compile for things the standard library already handles.
