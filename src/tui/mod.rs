pub mod actions;
pub mod tree;
pub mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use crate::model::RepoInfo;
use tree::{DisplayRow, SortMode};

fn detect_lazygit() -> bool {
    std::process::Command::new("lazygit")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub list_state: ListState,
    pub detail_expanded: bool,
    pub should_quit: bool,
    pub scan_path: std::path::PathBuf,
    pub scan_options: crate::scanner::ScanOptions,
    pub home_dir: Option<std::path::PathBuf>,
    pub sort_mode: SortMode,
    pub display_rows: Vec<DisplayRow>,
    pub has_lazygit: bool,
}

impl App {
    pub fn new(
        repos: Vec<RepoInfo>,
        scan_path: std::path::PathBuf,
        scan_options: crate::scanner::ScanOptions,
        home_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let mut app = Self {
            repos,
            selected: 0,
            list_state: ListState::default(),
            detail_expanded: true,
            should_quit: false,
            scan_path,
            scan_options,
            home_dir,
            sort_mode: SortMode::Tree,
            display_rows: Vec::new(),
            has_lazygit: detect_lazygit(),
        };
        app.rebuild_display_rows();
        app.select_first_repo();
        app
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        let row = self.display_rows.get(self.selected)?;
        let idx = row.repo_index()?;
        self.repos.get(idx)
    }

    pub fn next(&mut self) {
        let len = self.display_rows.len();
        if len == 0 {
            return;
        }
        let mut pos = self.selected + 1;
        while pos < len {
            if self.display_rows[pos].repo_index().is_some() {
                self.selected = pos;
                self.list_state.select(Some(pos));
                return;
            }
            pos += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected == 0 {
            return;
        }
        let mut pos = self.selected - 1;
        loop {
            if self.display_rows[pos].repo_index().is_some() {
                self.selected = pos;
                self.list_state.select(Some(pos));
                return;
            }
            if pos == 0 {
                break;
            }
            pos -= 1;
        }
    }

    pub fn toggle_detail(&mut self) {
        self.detail_expanded = !self.detail_expanded;
    }

    pub fn toggle_sort(&mut self) {
        let current_repo_index = self
            .display_rows
            .get(self.selected)
            .and_then(|r| r.repo_index());

        self.sort_mode = match self.sort_mode {
            SortMode::Tree => SortMode::Dirty,
            SortMode::Dirty => SortMode::Tree,
        };

        self.rebuild_display_rows();

        if let Some(repo_idx) = current_repo_index {
            for (i, row) in self.display_rows.iter().enumerate() {
                if row.repo_index() == Some(repo_idx) {
                    self.selected = i;
                    self.list_state.select(Some(i));
                    return;
                }
            }
        }

        self.select_first_repo();
    }

    pub fn refresh_all(&mut self) {
        let current_path = self.selected_repo().map(|r| r.path.clone());

        let repo_paths = crate::scanner::scan_repos(&self.scan_path, &self.scan_options);
        self.repos = repo_paths
            .iter()
            .filter_map(|p| crate::git::inspect_repo(p).ok())
            .collect();
        self.repos.sort_by_key(|r| r.risk_level());

        self.rebuild_display_rows();

        if let Some(path) = current_path {
            for (i, row) in self.display_rows.iter().enumerate() {
                if let Some(idx) = row.repo_index() {
                    if self.repos[idx].path == path {
                        self.selected = i;
                        self.list_state.select(Some(i));
                        return;
                    }
                }
            }
        }

        self.select_first_repo();
    }

    /// Re-sort repos and rebuild display rows, preserving selection on the given path.
    pub fn resort_and_reselect(&mut self, target_path: &std::path::Path) {
        self.repos.sort_by_key(|r| r.risk_level());
        self.rebuild_display_rows();

        for (i, row) in self.display_rows.iter().enumerate() {
            if let Some(idx) = row.repo_index() {
                if self.repos[idx].path == *target_path {
                    self.selected = i;
                    self.list_state.select(Some(i));
                    return;
                }
            }
        }

        self.select_first_repo();
    }

    fn rebuild_display_rows(&mut self) {
        self.display_rows = match self.sort_mode {
            SortMode::Tree => tree::build_tree_rows(&self.repos, &self.scan_path),
            SortMode::Dirty => tree::build_flat_rows(&self.repos, self.home_dir.as_deref()),
        };
    }

    fn select_first_repo(&mut self) {
        for (i, row) in self.display_rows.iter().enumerate() {
            if row.repo_index().is_some() {
                self.selected = i;
                self.list_state.select(Some(i));
                return;
            }
        }
        self.selected = 0;
        self.list_state.select(None);
    }
}

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut *app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                handle_key(key, app, terminal)?;
            }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Enter => app.toggle_detail(),
        KeyCode::Char('o') => app.toggle_sort(),
        KeyCode::Char('r') => app.refresh_all(),
        KeyCode::Char('s') => actions::open_shell(app, terminal)?,
        KeyCode::Char('e') => actions::open_editor(app, terminal)?,
        KeyCode::Char('c') => actions::launch_claude(app, terminal, false)?,
        KeyCode::Char('C') => actions::launch_claude(app, terminal, true)?,
        KeyCode::Char('p') => actions::git_push(app)?,
        KeyCode::Char('f') => actions::git_fetch(app)?,
        KeyCode::Char('P') => actions::git_pull(app)?,
        KeyCode::Char('y') => actions::copy_path(app)?,
        KeyCode::Char('l') => actions::launch_lazygit(app, terminal)?,
        _ => {}
    }
    Ok(())
}
