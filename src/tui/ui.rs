use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::model::RiskLevel;
use super::App;

/// Main draw entry point called every tick by the event loop.
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(if app.detail_expanded { 12 } else { 0 }),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_repo_list(f, app, chunks[1]);
    if app.detail_expanded {
        draw_detail(f, app, chunks[2]);
    }
    draw_footer(f, app, chunks[3]);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn display_path(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home
        && let Ok(stripped) = path.strip_prefix(home) {
            return format!("~/{}", stripped.display());
        }
    path.display().to_string()
}

fn risk_color(level: RiskLevel) -> Color {
    match level {
        RiskLevel::AtRisk => Color::Red,
        RiskLevel::Warning => Color::Yellow,
        RiskLevel::Safe => Color::Green,
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(
        "Grove — {}",
        display_path(&app.scan_path, app.home_dir.as_deref())
    );

    let at_risk = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::AtRisk).count();
    let warning = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::Warning).count();
    let safe = app.repos.iter().filter(|r| r.risk_level() == RiskLevel::Safe).count();

    let summary = Line::from(vec![
        Span::raw("Repositories ("),
        Span::styled(format!("{at_risk}"), Style::default().fg(Color::Red)),
        Span::raw(" at risk, "),
        Span::styled(format!("{warning}"), Style::default().fg(Color::Yellow)),
        Span::raw(" warning, "),
        Span::styled(format!("{safe}"), Style::default().fg(Color::Green)),
        Span::raw(" safe)"),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(summary).block(block);
    f.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Repo list
// ---------------------------------------------------------------------------

fn draw_repo_list(f: &mut Frame, app: &mut App, area: Rect) {
    // Compute column widths from actual data
    let mut path_w = 4_usize;
    let mut branch_w = 6_usize;
    let mut status_w = 6_usize;
    for repo in &app.repos {
        path_w = path_w.max(display_path(&repo.path, app.home_dir.as_deref()).len());
        branch_w = branch_w.max(repo.branch_display().len());
        status_w = status_w.max(repo.status_summary().len());
    }

    let items: Vec<ListItem> = app
        .repos
        .iter()
        .map(|repo| {
            let color = risk_color(repo.risk_level());
            let path_str = display_path(&repo.path, app.home_dir.as_deref());

            let line = Line::from(vec![
                Span::styled(
                    format!("{:<w$}", path_str, w = path_w),
                    Style::default().fg(color),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{:<w$}", repo.branch_display(), w = branch_w),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{:<w$}", repo.status_summary(), w = status_w),
                    Style::default().fg(color),
                ),
                Span::raw("  "),
                Span::styled(
                    repo.sync_summary(),
                    Style::default().fg(color),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Repos");

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▸ ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, area, &mut app.list_state);

    // Scrollbar
    if app.repos.len() > area.height.saturating_sub(2) as usize {
        let mut scrollbar_state = ScrollbarState::new(app.repos.len())
            .position(app.selected);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        f.render_stateful_widget(scrollbar, area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }), &mut scrollbar_state);
    }
}

// ---------------------------------------------------------------------------
// Detail panel
// ---------------------------------------------------------------------------

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let repo = match app.selected_repo() {
        Some(r) => r,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Details");
            let paragraph = Paragraph::new("No repository selected");
            f.render_widget(paragraph.block(block), area);
            return;
        }
    };

    let path_str = display_path(&repo.path, app.home_dir.as_deref());
    let title = format!("{} ({})", path_str, repo.branch_display());

    let mut lines: Vec<Line> = Vec::new();

    let dirty = repo.modified_count + repo.staged_count;
    if dirty > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} modified/staged files", dirty),
            Style::default().fg(Color::Red),
        )));
    }

    if repo.untracked_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} untracked files", repo.untracked_count),
            Style::default().fg(Color::Red),
        )));
    }

    if repo.ahead > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} unpushed commits", repo.ahead),
            Style::default().fg(Color::Red),
        )));
    }

    if repo.behind > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} commits behind remote", repo.behind),
            Style::default().fg(Color::Yellow),
        )));
    }

    if repo.stash_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} stashes", repo.stash_count),
            Style::default().fg(Color::Red),
        )));
    }

    if !repo.has_remote {
        lines.push(Line::from(Span::styled(
            "  No remote configured",
            Style::default().fg(Color::Red),
        )));
    }

    if repo.has_remote && !repo.has_upstream && !repo.is_detached {
        lines.push(Line::from(Span::styled(
            "  Branch has no upstream tracking",
            Style::default().fg(Color::Red),
        )));
    }

    if repo.merge_in_progress {
        lines.push(Line::from(Span::styled(
            "  \u{26a0} Merge in progress",
            Style::default().fg(Color::Red),
        )));
    }

    if repo.rebase_in_progress {
        lines.push(Line::from(Span::styled(
            "  \u{26a0} Rebase in progress",
            Style::default().fg(Color::Red),
        )));
    }

    if repo.is_detached {
        lines.push(Line::from(Span::styled(
            "  Detached HEAD",
            Style::default().fg(Color::Yellow),
        )));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  \u{2713} All clean \u{2014} fully synced with remote",
            Style::default().fg(Color::Green),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    // Context-sensitive keys based on selected repo
    if let Some(repo) = app.selected_repo() {
        if repo.ahead > 0 && repo.has_remote {
            append_key_hint(&mut spans, "p", "ush");
        }
        if repo.behind > 0 {
            append_key_hint(&mut spans, "P", "ull");
        }
        if repo.has_remote {
            append_key_hint(&mut spans, "f", "etch");
        }
        if repo.stash_count > 0 {
            append_key_hint(&mut spans, "t", "stash");
        }
    }

    // Always-present keys
    append_key_hint(&mut spans, "s", "hell");
    append_key_hint(&mut spans, "e", "ditor");
    append_key_hint(&mut spans, "c", "laude");
    append_key_hint(&mut spans, "r", "efresh");
    append_key_hint(&mut spans, "q", "uit");

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

/// Append a styled key hint like "[k]ey" to the spans vector, with a trailing space separator.
fn append_key_hint<'a>(spans: &mut Vec<Span<'a>>, key: &'a str, rest: &'a str) {
    if !spans.is_empty() {
        spans.push(Span::raw(" "));
    }
    spans.push(Span::raw("["));
    spans.push(Span::styled(key, Style::default().fg(Color::Cyan)));
    spans.push(Span::raw("]"));
    spans.push(Span::raw(rest));
}
