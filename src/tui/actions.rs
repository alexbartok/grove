use std::io;
use std::process::Command;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::App;

/// Re-inspect a single repo and update app state.
fn refresh_repo(app: &mut App, path: &std::path::Path) {
    if let Ok(updated) = crate::git::inspect_repo(path)
        && let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
            *repo = updated;
        }
    app.resort_and_reselect(path);
}

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
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let path = info.path.clone();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".into());
    let mut cmd = Command::new(&shell);
    cmd.current_dir(&path);
    suspend_and_run(terminal, cmd)?;
    refresh_repo(app, &path);
    Ok(())
}

pub fn open_editor(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let path = info.path.clone();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let mut cmd = Command::new(&editor);
    cmd.arg(".").current_dir(&path);
    suspend_and_run(terminal, cmd)?;
    refresh_repo(app, &path);
    Ok(())
}

pub fn launch_claude(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    dangerously_skip_permissions: bool,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    let path = info.path.clone();
    let mut cmd = Command::new("claude");
    if dangerously_skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    cmd.current_dir(&path);
    suspend_and_run(terminal, cmd)?;
    refresh_repo(app, &path);
    Ok(())
}

pub fn launch_lazygit(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if !app.has_lazygit { return Ok(()) }
    let path = info.path.clone();
    let mut cmd = Command::new("lazygit");
    cmd.current_dir(&path);
    suspend_and_run(terminal, cmd)?;
    refresh_repo(app, &path);
    Ok(())
}

/// Run a git command, set a flash message on failure.
fn run_git(app: &mut App, path: &std::path::Path, args: &[&str]) {
    match Command::new("git").args(args).current_dir(path).output() {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.lines().next().unwrap_or("unknown error");
            app.flash_message = Some((
                format!("git {} failed: {}", args[0], msg),
                std::time::Instant::now(),
            ));
        }
        Err(e) => {
            app.flash_message = Some((
                format!("git {}: {}", args[0], e),
                std::time::Instant::now(),
            ));
        }
        _ => {}
    }
}

pub fn git_push(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.ahead == 0 || !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    run_git(app, &path, &["push"]);
    refresh_repo(app, &path);
    Ok(())
}

pub fn git_fetch(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if !info.has_remote { return Ok(()) }

    let path = info.path.clone();
    run_git(app, &path, &["fetch"]);
    refresh_repo(app, &path);
    Ok(())
}

pub fn git_pull(app: &mut App) -> Result<()> {
    let Some(info) = app.selected_repo() else { return Ok(()) };
    if info.behind == 0 { return Ok(()) }

    let path = info.path.clone();
    run_git(app, &path, &["pull"]);
    refresh_repo(app, &path);
    Ok(())
}

pub fn copy_path(app: &mut App) -> Result<()> {
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
        // Try clipboard tools in order: wl-copy (Wayland), xclip, xsel (X11)
        let clipboard_cmds: &[(&str, &[&str])] = &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ];
        let mut copied = false;
        for (cmd, args) in clipboard_cmds {
            if let Ok(mut child) = Command::new(cmd)
                .args(*args)
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(path_str.as_bytes())?;
                }
                child.wait()?;
                copied = true;
                break;
            }
        }
        if !copied {
            app.flash_message = Some((
                "No clipboard tool found (install xclip, xsel, or wl-copy)".into(),
                std::time::Instant::now(),
            ));
        }
    }

    Ok(())
}
