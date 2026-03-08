# Grove

A CLI tool that answers: **"If this machine died right now, would I lose any work?"**

Grove scans a directory tree for git repositories and shows their safety status at a glance. Repos with uncommitted changes, unpushed commits, missing remotes, or stashes are surfaced immediately — colored red so you can't miss them.

## Install

```
cargo install --path .
```

Requires Rust 1.85+ (2024 edition).

## Usage

```
grove [OPTIONS] [PATH]
```

By default, Grove launches an interactive TUI. If stdout is not a TTY (e.g. piped), it falls back to static columnar output.

### Options

| Flag | Description |
|------|-------------|
| `[PATH]` | Directory to scan (default: `.`) |
| `-n`, `--no-interactive` | Force static output mode |
| `-H`, `--hidden` | Include hidden directories in traversal |
| `-d`, `--max-depth <N>` | Limit directory traversal depth |
| `--all-filesystems` | Cross filesystem boundaries |

### Examples

```bash
# Scan your home projects directory
grove ~/projects

# Quick check, no TUI
grove -n ~/projects

# Include dotfile repos
grove -H ~

# Scan only two levels deep
grove -d 2 ~/code
```

## Static output

```
REPO                    BRANCH    STATUS       STASH  REMOTE    SYNC
~/projects/frontend     feat/nav  2 modified   —      origin    ↑3 ahead
~/projects/dotfiles     main      ✓ clean      —      —         ✗ no remote
~/projects/scripts      main      1 untracked  1      origin    ✓ synced
~/projects/api-server   main      ✓ clean      —      origin    ✓ synced
```

Rows are color-coded: red for repos with at-risk data, yellow for unusual states (detached HEAD), green for fully synced repos. Sorted by risk — problems float to the top.

## Interactive TUI

The TUI shows all repos in a navigable list with a detail panel for the selected repo.

### Keybindings

| Key | Action | When |
|-----|--------|------|
| `↑`/`↓`, `j`/`k` | Navigate repos | Always |
| `Enter` | Toggle detail panel | Always |
| `s` | Open shell in repo | Always |
| `e` | Open `$EDITOR` in repo | Always |
| `c` | Launch `claude` in repo | Always |
| `C` | Launch `claude --dangerously-skip-permissions` | Always |
| `p` | Git push | When ahead of remote |
| `f` | Git fetch | When remote exists |
| `P` | Git pull | When behind remote |
| `y` | Copy repo path to clipboard | Always |
| `r` | Refresh all repos | Always |
| `q` / `Esc` | Quit | Always |

Context-sensitive keys only appear in the footer when they're relevant to the selected repo.

## What Grove checks

| Condition | Meaning |
|-----------|---------|
| Uncommitted changes | Modified or staged files not in any commit |
| Untracked files | Files git doesn't know about |
| No remote configured | Entire repo is local-only |
| Unpushed commits | Commits that exist nowhere else |
| Stashes | Stashes are local-only |
| No upstream tracking | Branch may have no remote counterpart |
| Merge/rebase in progress | Incomplete operation |
| Detached HEAD | Not on a branch — easy to lose commits |

## How traversal works

- Walks directories recursively looking for `.git` directories
- When `.git` is found, the repo is recorded and Grove does not descend further into it
- Hidden directories (starting with `.`) are skipped by default — use `-H` to include them
- Stops at filesystem boundaries (mount points) by default — use `--all-filesystems` to cross them
- All repos are discovered and inspected before any output is shown

## License

MIT
