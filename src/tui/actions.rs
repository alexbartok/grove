use std::io;
use std::process::Command;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::App;

/// Suspend TUI, run a command, then restore TUI.
fn suspend_and_run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut cmd: Command,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    let status = cmd.status();

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    if let Err(e) = status {
        eprintln!("Command failed: {}", e);
    }

    Ok(())
}

pub fn open_shell(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let mut cmd = Command::new(&shell);
    cmd.current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn open_editor(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let mut cmd = Command::new(&editor);
    cmd.arg(".").current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn launch_claude(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    dangerously_skip_permissions: bool,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let mut cmd = Command::new("claude");
    if dangerously_skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    cmd.current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn launch_lazygit(
    app: &App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if !app.has_lazygit { return Ok(()) }
    let mut cmd = Command::new("lazygit");
    cmd.current_dir(&info.path);
    suspend_and_run(terminal, cmd)
}

pub fn git_push(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.ahead == 0 || !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["push"])
        .current_dir(&path)
        .output()?;

    if let Ok(updated) = crate::git::inspect_repo(&path)
        && let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    app.resort_and_reselect(&path);
    Ok(())
}

pub fn git_fetch(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["fetch"])
        .current_dir(&path)
        .output()?;

    if let Ok(updated) = crate::git::inspect_repo(&path)
        && let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    app.resort_and_reselect(&path);
    Ok(())
}

pub fn git_pull(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.behind == 0 { return Ok(()) }

    let path = info.path.clone();
    Command::new("git")
        .args(["pull"])
        .current_dir(&path)
        .output()?;

    if let Ok(updated) = crate::git::inspect_repo(&path)
        && let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    app.resort_and_reselect(&path);
    Ok(())
}

pub fn copy_path(app: &App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let path_str = info.path.display().to_string();

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(path_str.as_bytes())?;
        }
        child.wait()?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        use std::io::Write;
        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(path_str.as_bytes())?;
            }
            child.wait()?;
        }
    }

    Ok(())
}
