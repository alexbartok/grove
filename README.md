# Grove

Scans a directory tree for git repositories and shows you which ones have uncommitted work, unpushed commits, or no remote. If your machine died right now, Grove tells you what you'd lose.

## Install

```
cargo install --path .
```

Requires Rust 1.85+ (2024 edition).

## Usage

```
grove [OPTIONS] [PATH]
```

Opens an interactive TUI by default. When stdout isn't a TTY (piped, redirected), it prints static columnar output instead.

### Options

| Flag | Description |
|------|-------------|
| `[PATH]` | Directory to scan (default: `.`) |
| `-n`, `--no-interactive` | Static output, no TUI |
| `-H`, `--hidden` | Include hidden directories |
| `-d`, `--max-depth <N>` | Limit traversal depth |
| `--all-filesystems` | Cross filesystem boundaries |

### Examples

```bash
grove ~/projects
grove -n ~/projects
grove -H ~
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

Rows are colored red/yellow/green based on whether there's local-only data. Dirty repos sort to the top.

## Interactive TUI

A scrollable repo list with a detail panel for the selected repo.

### Keybindings

| Key | Action | When shown |
|-----|--------|------------|
| `↑`/`↓`, `j`/`k` | Navigate | Always |
| `Enter` | Toggle detail panel | Always |
| `s` | Shell in repo dir | Always |
| `e` | `$EDITOR` in repo dir | Always |
| `c` | `claude` in repo dir | Always |
| `C` | `claude --dangerously-skip-permissions` | Always |
| `p` | `git push` | Ahead of remote |
| `f` | `git fetch` | Has remote |
| `P` | `git pull` | Behind remote |
| `y` | Copy path to clipboard | Always |
| `r` | Refresh | Always |
| `q` / `Esc` | Quit | Always |

Keys for push/pull/fetch only appear when they'd do something useful.

## What it checks

For each repo, Grove looks at:

- Uncommitted changes (modified, staged, untracked files)
- Whether a remote is configured
- Unpushed commits
- Stash entries
- Whether the branch tracks an upstream
- Merge or rebase in progress
- Detached HEAD

## Traversal

Grove walks directories recursively looking for `.git`. When it finds one, it records the repo and doesn't descend further. Hidden directories are skipped unless you pass `-H`. Traversal stops at filesystem boundaries (mount points) unless you pass `--all-filesystems`.

## License

MIT
