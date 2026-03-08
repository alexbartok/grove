# Contributing

## Setup

```bash
git clone <repo-url>
cd grove
cargo build
cargo test
```

Requires Rust 1.85+ (edition 2024).

## Project structure

```
src/
├── main.rs              Entry point, CLI parsing, mode dispatch
├── lib.rs               Module exports
├── model.rs             Core types: RepoInfo, RiskLevel
├── scanner.rs           Filesystem traversal, repo discovery
├── git.rs               Git status inspection (git2)
├── static_output.rs     Non-interactive columnar output
└── tui/
    ├── mod.rs           App state, event loop, key handling
    ├── ui.rs            TUI layout and rendering (ratatui)
    └── actions.rs       TUI action handlers
```

See `docs/ARCHITECTURE.md` for detailed module descriptions and data flow.

## Testing

```bash
cargo test              # run all tests
cargo test --lib model  # run just model tests
cargo test --lib git    # run just git inspection tests
```

Tests in `scanner.rs` and `git.rs` create real (temporary) git repos using `tempfile` and `git2`. No mocks — tests exercise actual git operations.

## Code style

- Run `cargo clippy -- -W clippy::all` before committing
- No warnings policy
- Keep dependencies minimal — prefer stdlib when reasonable
- Error handling: `anyhow::Result` for application-level errors, graceful degradation for individual repo inspection failures

## Adding a new TUI action

1. Add the handler function in `src/tui/actions.rs`
2. Add the keybinding in `handle_key()` in `src/tui/mod.rs`
3. Add the context-sensitive footer hint in `draw_footer()` in `src/tui/ui.rs`
4. If the action modifies repo state, re-inspect and re-sort after completion

## Adding a new status indicator

1. Add the field to `RepoInfo` in `src/model.rs`
2. Update `risk_level()` classification if it affects risk
3. Populate it in `inspect_repo()` in `src/git.rs`
4. Update `status_summary()` or `sync_summary()` for static display
5. Add it to the detail panel in `draw_detail()` in `src/tui/ui.rs`
6. Add tests
