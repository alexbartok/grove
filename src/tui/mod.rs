pub mod actions;
pub mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::model::RepoInfo;

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub detail_expanded: bool,
    pub should_quit: bool,
    pub scan_path: std::path::PathBuf,
    pub scan_options: crate::scanner::ScanOptions,
    pub home_dir: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(
        repos: Vec<RepoInfo>,
        scan_path: std::path::PathBuf,
        scan_options: crate::scanner::ScanOptions,
        home_dir: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            repos,
            selected: 0,
            detail_expanded: true,
            should_quit: false,
            scan_path,
            scan_options,
            home_dir,
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        self.repos.get(self.selected)
    }

    pub fn next(&mut self) {
        if !self.repos.is_empty() {
            self.selected = (self.selected + 1).min(self.repos.len() - 1);
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_detail(&mut self) {
        self.detail_expanded = !self.detail_expanded;
    }

    pub fn refresh_all(&mut self) {
        let repo_paths = crate::scanner::scan_repos(&self.scan_path, &self.scan_options);
        self.repos = repo_paths
            .iter()
            .filter_map(|p| crate::git::inspect_repo(p).ok())
            .collect();
        self.repos.sort_by_key(|r| r.risk_level());
        if self.selected >= self.repos.len() {
            self.selected = self.repos.len().saturating_sub(1);
        }
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
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(key, app, terminal)?;
            }
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
        KeyCode::Char('r') => app.refresh_all(),
        KeyCode::Char('s') => actions::open_shell(app, terminal)?,
        KeyCode::Char('e') => actions::open_editor(app, terminal)?,
        KeyCode::Char('c') => actions::launch_claude(app, terminal, false)?,
        KeyCode::Char('C') => actions::launch_claude(app, terminal, true)?,
        KeyCode::Char('p') => actions::git_push(app)?,
        KeyCode::Char('f') => actions::git_fetch(app)?,
        KeyCode::Char('P') => actions::git_pull(app)?,
        KeyCode::Char('y') => actions::copy_path(app)?,
        _ => {}
    }
    Ok(())
}
