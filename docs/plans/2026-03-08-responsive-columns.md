# Responsive Columns Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers-extended-cc:executing-plans to implement this plan task-by-task.

**Goal:** Make TUI columns adapt to terminal width — truncate names, hide low-priority columns, let user toggle column visibility.

**Architecture:** `draw_repo_list` becomes width-aware. Compute ideal widths, fit to available space by truncating name column first, then hiding columns right-to-left. User can toggle columns with a key.

**Tech Stack:** No new deps. Changes only in `src/tui/ui.rs` and `src/tui/mod.rs`.

---

## Design

### Column priority (highest to lowest)
1. REPO (name) — always shown, truncatable with `...`
2. BRANCH — always shown
3. STATUS — always shown
4. STASH — hideable
5. REMOTE — hideable
6. SYNC — hideable (often empty anyway)

### Fitting algorithm in draw_repo_list
1. Compute ideal width for each column from data
2. Sum all columns + separators (2 chars each) + highlight symbol (2 chars)
3. Compare to `inner.width`
4. If fits: render as-is
5. If doesn't fit: shrink name column (min 10 chars, truncate with `...`)
6. If still doesn't fit: hide SYNC column, recalculate
7. If still doesn't fit: hide REMOTE column, recalculate
8. If still doesn't fit: hide STASH column, recalculate
9. Column header row must match visible columns

### Column visibility toggle
- Add `visible_columns: HashSet<Column>` to App (or a bitflag)
- `Column` enum: `Repo, Branch, Status, Stash, Remote, Sync`
- Default: all visible (subject to auto-hiding from width)
- Key: maybe `+`/`-` or a column picker submenu — TBD
- Simpler alternative: just auto-hide, no manual toggle. Show indicator like "Repos (tree, 3/6 cols)" in the block title so user knows columns are hidden.

### Name truncation helper
```rust
fn truncate_name(name: &str, max_width: usize) -> String {
    if name.chars().count() <= max_width {
        name.to_string()
    } else {
        let truncated: String = name.chars().take(max_width.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}
```

---

## Tasks

### Task 1: Width-aware column rendering
**Files:** `src/tui/ui.rs`

- Add `truncate_name` helper
- In `draw_repo_list`, after computing ideal widths, compare total to `inner.width`
- Shrink name column if needed
- Track which columns are visible in a local vec/flags
- Render only visible columns + matching header
- Show "(N cols hidden)" in block title when columns are dropped

### Task 2: Column header tracks visibility
**Files:** `src/tui/ui.rs`

- Header row only shows labels for visible columns
- Separator spacing matches data rows

### Task 3: Test edge cases
- Very narrow terminal (< 40 cols)
- Very wide terminal (all columns fit)
- Medium terminal (name truncated but all columns visible)
- Narrow terminal (columns hidden)

## Implementation order
1. Task 1 (the core logic)
2. Task 2 (header alignment — likely handled in Task 1)
3. Task 3 (manual testing + verify)
