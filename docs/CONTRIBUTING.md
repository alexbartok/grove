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
├── main.rs              CLI parsing, mode dispatch
├── lib.rs               Module exports
├── model.rs             RepoInfo, RiskLevel
├── scanner.rs           Filesystem walk, repo discovery
├── git.rs               Git inspection (git2)
├── static_output.rs     Columnar output
└── tui/
    ├── mod.rs           App state, event loop, keys
    ├── ui.rs            Layout and rendering (ratatui)
    └── actions.rs       Action handlers
```

See `docs/ARCHITECTURE.md` for how the pieces fit together.

## Testing

```bash
cargo test
cargo test --lib model
cargo test --lib git
```

Tests in `scanner.rs` and `git.rs` create temporary git repos on disk using `tempfile` and `git2`. No mocks.

## Code style

- `cargo clippy -- -W clippy::all` should be clean
- Prefer stdlib over adding a crate
- `anyhow::Result` for errors. If inspecting a single repo fails, warn and skip it rather than crashing

## Adding a TUI action

1. Write the handler in `src/tui/actions.rs`
2. Add the key in `handle_key()` in `src/tui/mod.rs`
3. Add the footer hint in `draw_footer()` in `src/tui/ui.rs`
4. If it changes repo state, re-inspect and re-sort afterward

## Adding a status indicator

1. Add the field to `RepoInfo` in `src/model.rs`
2. Update `risk_level()` if it affects the classification
3. Populate it in `inspect_repo()` in `src/git.rs`
4. Show it in `status_summary()` or `sync_summary()` for static output
5. Show it in `draw_detail()` in `src/tui/ui.rs`
6. Add tests
