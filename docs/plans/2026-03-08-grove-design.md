# Grove — Design Document

## Purpose

A CLI tool that answers: *"If this machine died right now, would I lose any work?"*

Grove scans a directory tree for git repositories and shows their safety status at a glance, with interactive capabilities to fix problems.

## Modes

- **Interactive (default):** Full-screen TUI. List of repos with detail panel. Navigate, inspect, and take actions.
- **Static (`-n` / `--no-interactive` or piped output):** Columnar output, pipe-friendly, suitable for scripts. Auto-detected when stdout is not a TTY.

## CLI Interface

```
grove [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to scan (default: current directory)

Options:
  -n, --no-interactive       Static output mode (auto if not a TTY)
  -H, --hidden               Include hidden directories in traversal
  -d, --max-depth <N>        Maximum directory traversal depth
      --all-filesystems      Cross filesystem boundaries
  -h, --help                 Print help
  -V, --version              Print version
```

### Defaults

- Interactive mode when stdout is a TTY, static otherwise.
- Hidden directories are skipped (except `.git` detection).
- Traversal stops at filesystem boundaries.
- No depth limit.

## Status Model

Every repo is classified by **risk level** based on whether local-only data exists.

### Risk: At Risk (red)

Data would be lost if the machine died.

| Indicator | Meaning |
|---|---|
| Uncommitted changes (staged/unstaged) | Modified files not in any commit |
| Untracked files | Files git doesn't know about |
| No remote configured | Entire repo is local-only |
| Unpushed commits | Commits that exist nowhere else |
| Stashes | Stashes are local-only, always lost |
| Branch has no upstream tracking | Branch may have no remote counterpart |
| Merge/rebase in progress | Work in limbo, incomplete operation |

### Risk: Warning (yellow)

Not directly at risk, but unusual state worth investigating.

| Indicator | Meaning |
|---|---|
| Detached HEAD | Not on a branch, easy to lose commits |

### Risk: Safe (green)

All data exists upstream. Nothing to lose.

| Indicator | Meaning |
|---|---|
| Clean working tree + pushed + remote configured | Fully backed up |

## Static Output

Columnar format, color-coded by risk level:

```
REPO                    BRANCH    STATUS       STASH  REMOTE    SYNC
~/projects/api-server   main      ✓ clean      —      origin    ✓ synced
~/projects/frontend     feat/nav  2 modified   —      origin    ↑3 ahead
~/projects/dotfiles     main      ✓ clean      —      —         ✗ no remote
~/projects/scripts      main      1 untracked  1      origin    ✓ synced
~/projects/lib          main      ✓ clean      —      origin    ⚠ no tracking
~/projects/experiment   HEAD      ✓ clean      —      origin    ✓ synced
```

- Green rows: safe repos.
- Red rows/fields: at-risk indicators.
- Yellow rows/fields: warnings.
- Repos sorted by risk level (at-risk first, safe last).

## Interactive TUI

### Layout

```
┌─ Grove ─ ~/projects ──────────────────────────────────────────┐
│                                                                │
│  Repositories (4 at risk, 1 warning, 2 clean)                 │
│                                                                │
│  ▸ ~/projects/frontend        feat/nav   2 modified  ↑3 ahead │
│    ~/projects/dotfiles        main       ✓ clean     ✗ no rem │
│    ~/projects/scripts         main       1 untracked  1 stash │
│    ~/projects/lib             main       ✓ clean     ⚠ no trk │
│    ~/projects/experiment      HEAD       ✓ clean     ✓ synced │
│    ~/projects/api-server      main       ✓ clean     ✓ synced │
│                                                                │
│ ┌─ Detail ────────────────────────────────────────────────────┐│
│ │ ~/projects/frontend (feat/nav)                              ││
│ │                                                             ││
│ │ Modified files (2):                                         ││
│ │   M src/components/Nav.tsx                                  ││
│ │   M src/styles/nav.css                                      ││
│ │                                                             ││
│ │ Unpushed commits (3):                                       ││
│ │   a1b2c3d  fix nav alignment                                ││
│ │   d4e5f6a  add mobile breakpoints                           ││
│ │   b7c8d9e  refactor nav component                           ││
│ └─────────────────────────────────────────────────────────────┘│
│                                                                │
│ [p]ush  [s]hell  [c]laude  [q]uit                    ↑↓ navigate│
└────────────────────────────────────────────────────────────────┘
```

### Sorting

At-risk repos sort to the top. Safe repos sink to the bottom. Problems are always visible first.

### Keybindings

Context-sensitive — only shown/active when relevant to the selected repo's state.

| Key | Action | When |
|---|---|---|
| `↑`/`↓`, `j`/`k` | Navigate repos | Always |
| `Enter` | Expand/collapse detail | Always |
| `s` | Open shell in repo directory | Always |
| `e` | Open repo in `$EDITOR` | Always |
| `c` | Launch `claude` in repo | Always |
| `C` | Launch `claude --dangerously-skip-permissions` | Always |
| `p` | Git push | When ahead of remote |
| `f` | Git fetch + refresh status | When remote exists |
| `P` | Git pull | When behind remote |
| `t` | List/pop/drop stashes | When stashes exist |
| `a` | Add remote (prompt for URL) | When no remote |
| `r` | Refresh all repos | Always |
| `y` | Copy repo path to clipboard | Always |
| `q` / `Esc` | Quit | Always |

### Action Behavior

Actions that modify state (push, pull, stash operations) refresh the affected repo's status after completion. Actions that leave grove (shell, editor, claude) suspend the TUI, run the command, and restore the TUI on return.

## Filesystem Traversal

- Walk directories recursively looking for `.git` directories.
- When `.git` found, record the repo and do not descend further into it (but continue scanning sibling directories).
- By default, skip hidden directories (names starting with `.`). `--hidden` / `-H` includes them.
- By default, detect filesystem boundaries via device ID (`stat`) and stop at mount points. `--all-filesystems` disables this check.
- `--max-depth` / `-d` limits recursion depth relative to the starting path.

## Tech Stack

| Crate | Purpose |
|---|---|
| `clap` | CLI argument parsing with derive macros |
| `git2` | Native git operations (libgit2 bindings) |
| `ratatui` | TUI framework |
| `crossterm` | Terminal backend for ratatui |
| `colored` | Static output coloring |

## Architecture

```
grove/
├── src/
│   ├── main.rs              # Entry point, arg parsing, mode dispatch
│   ├── scanner.rs           # Filesystem traversal, repo discovery
│   ├── git.rs               # Git status inspection per repo
│   ├── model.rs             # RepoStatus, RiskLevel, RepoInfo types
│   ├── static_output.rs     # Columnar printer for static mode
│   ├── tui/
│   │   ├── mod.rs           # TUI app state + event loop
│   │   ├── ui.rs            # Layout + rendering
│   │   └── actions.rs       # Action handlers (push, shell, claude, etc.)
│   └── lib.rs               # Shared exports
├── Cargo.toml
└── docs/
    └── plans/
```

### Data Flow

1. `main.rs` parses args, determines mode (interactive/static).
2. `scanner.rs` walks the filesystem, yields repo paths.
3. `git.rs` inspects each repo path, produces `RepoInfo` with full status.
4. `model.rs` classifies risk level, sorts repos.
5. Either `static_output.rs` prints and exits, or `tui/` launches the interactive app.
6. In TUI mode, `actions.rs` handles user commands, calls back into `git.rs` for refreshes.

### Key Design Decisions

- **git2 over shelling out:** Native bindings are faster, more reliable, and don't require git to be installed on the target machine.
- **Scan-then-display:** All repos are discovered and inspected before rendering. For large trees, a progress indicator is shown during the scan phase.
- **No config file:** All behavior controlled via CLI flags. Keep it simple.
- **Suspend/resume for external commands:** Shell, editor, and claude actions cleanly suspend the TUI rather than trying to embed a terminal.
