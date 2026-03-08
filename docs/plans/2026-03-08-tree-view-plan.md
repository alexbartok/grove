# Tree View & UI Polish Plan

## Changes

### 1. Simplify header
- Replace "15 at risk, 0 warning, 11 safe" with "26 repos, 15 dirty"
- "dirty" = any repo that's not RiskLevel::Safe
- Files: `src/tui/ui.rs` (draw_header)

### 2. Add SortMode and DisplayRow types
- `SortMode::Tree` (default) — alphabetical tree view
- `SortMode::Dirty` — dirty repos first, flat list (current behavior)
- `DisplayRow` enum:
  - `Directory { name: String, tree_prefix: String }` — non-selectable group header
  - `Repo { repo_index: usize, display_name: String, tree_prefix: String }` — selectable
- Files: `src/tui/mod.rs`

### 3. Tree building logic
- For each repo, compute path relative to `scan_path`
- Build a trie: intermediate nodes are directories, leaves are repos
- Flatten with tree-line prefixes:
  - Not-last child: connector `├── `, continuation for children `│   `
  - Last child: connector `└── `, continuation for children `    `
- Sort children alphabetically at each level
- A node that is both a repo AND has children: show as Repo row, then recurse children

Example output with user's actual repos:
```
├── 7hauben-scraper            main    ✓ clean    ✓ synced
├── ard
│   ├── ard-plus-dl            main    ✓ clean    ✓ synced
│   └── ard-plus-dl-local      main    1 untracked ✓ synced
├── blinkist-downloader        main    ✓ clean    ✓ synced
├── claude-projects
│   ├── amex-restaurants       main    ✓ clean    ✗ no remote
│   ├── grove                  feat/.. 1 modified  ✗ no remote
│   └── kuma-cli               main    ✓ clean    ✗ no remote
├── claude-usage-tool          main    1 modified  ✓ synced
└── cv                         main    ✓ clean    ✓ synced
```

- Files: `src/tui/mod.rs` (or new `src/tui/tree.rs`)

### 4. Update App struct
- Add fields: `sort_mode: SortMode`, `display_rows: Vec<DisplayRow>`
- Add `rebuild_display_rows()` — called from `new()`, `refresh_all()`, `toggle_sort()`
- In Tree mode: build tree rows from repos
- In Dirty mode: flat list of Repo rows sorted by risk level
- Modify `next()`/`previous()` to skip Directory rows in display_rows
- Modify `selected_repo()` to use repo_index from current DisplayRow::Repo
- `list_state.select()` tracks position in display_rows (not repos)
- Files: `src/tui/mod.rs`

### 5. Update repo list rendering
- Iterate `display_rows` instead of `repos`
- Directory rows: show tree_prefix + name in dim/white, no status columns
- Repo rows: show tree_prefix + display_name (leaf name only), then branch/status/sync columns
- Column width computation uses display_name + tree_prefix length for name column
- Scrollbar content_length = display_rows.len() (not repos.len())
- Files: `src/tui/ui.rs` (draw_repo_list)

### 6. Add sort toggle keybinding
- `o` toggles between Tree and Dirty
- Calls `toggle_sort()` which flips mode and calls `rebuild_display_rows()`
- Preserves selected repo across toggle (find same repo_index in new display_rows)
- Files: `src/tui/mod.rs` (handle_key, App impl)

### 7. Update footer
- Add `[o]rder` hint, always shown
- Show current mode indicator: "tree" or "dirty" somewhere subtle
- Files: `src/tui/ui.rs` (draw_footer)

### 8. Update static output
- Also apply the header change (no risk labels) for consistency
- Static output keeps flat columnar format (no tree view needed there)
- Files: `src/static_output.rs` — minor, just drop risk category labels if any

## Implementation order
1. SortMode + DisplayRow types in mod.rs
2. Tree building function
3. rebuild_display_rows() + update App struct
4. Update next/previous/selected_repo to use display_rows
5. Update draw_repo_list to render display_rows
6. Update draw_header
7. Add keybinding + footer
8. Test both modes
