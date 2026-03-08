use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::model::RiskLevel;
use super::App;
use super::tree::{DisplayRow, SortMode};

/// Main draw entry point called every tick by the event loop.
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(if app.detail_expanded { 9 } else { 0 }),
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

    let total = app.repos.len();
    let dirty = app.repos.iter().filter(|r| r.risk_level() != RiskLevel::Safe).count();

    let summary = if dirty > 0 {
        Line::from(vec![
            Span::raw(format!("{total} repos, ")),
            Span::styled(format!("{dirty} dirty"), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{total} repos, all clean"), Style::default().fg(Color::Green)),
        ])
    };

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
    // Compute column widths from display_rows (min widths = header labels)
    let mut name_w = 4_usize;   // "REPO"
    let mut branch_w = 6_usize; // "BRANCH"
    let mut status_w = 6_usize; // "STATUS"
    let mut stash_w = 5_usize;  // "STASH"
    let mut remote_w = 6_usize; // "REMOTE"

    for row in &app.display_rows {
        let full_name_len = row.tree_prefix().len() + row.display_name().len();
        name_w = name_w.max(full_name_len);
        if let Some(idx) = row.repo_index() {
            if let Some(repo) = app.repos.get(idx) {
                branch_w = branch_w.max(repo.branch_display().len());
                status_w = status_w.max(repo.status_summary().len());
                stash_w = stash_w.max(repo.stash_summary().len());
                remote_w = remote_w.max(repo.remote_name.as_deref().unwrap_or("\u{2014}").len());
            }
        }
    }

    // Render the outer block first, then split inner area into header + list
    let mode_label = match app.sort_mode {
        SortMode::Tree => "tree",
        SortMode::Dirty => "dirty-first",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Repos ({mode_label})"));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let header_area = Rect { height: 1, ..inner };
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height - 1,
        ..inner
    };

    // Fixed column header (indented to align with highlight_symbol "▸ ")
    let header_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<w$}", "REPO", w = name_w),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<w$}", "BRANCH", w = branch_w),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<w$}", "STATUS", w = status_w),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<w$}", "STASH", w = stash_w),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<w$}", "REMOTE", w = remote_w),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled("SYNC", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(header_line), header_area);

    // Scrollable repo list
    let items: Vec<ListItem> = app
        .display_rows
        .iter()
        .map(|row| match row {
            DisplayRow::Directory { name, tree_prefix } => {
                let line = Line::from(vec![Span::styled(
                    format!("{tree_prefix}{name}"),
                    Style::default().fg(Color::DarkGray),
                )]);
                ListItem::new(line)
            }
            DisplayRow::Repo {
                repo_index,
                display_name,
                tree_prefix,
            } => {
                let repo = &app.repos[*repo_index];
                let color = risk_color(repo.risk_level());
                let prefix_and_name = format!("{tree_prefix}{display_name}");
                let remote_display = repo.remote_name.as_deref().unwrap_or("\u{2014}");

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<w$}", prefix_and_name, w = name_w),
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
                        format!("{:<w$}", repo.stash_summary(), w = stash_w),
                        Style::default().fg(color),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<w$}", remote_display, w = remote_w),
                        Style::default().fg(color),
                    ),
                    Span::raw("  "),
                    Span::styled(repo.sync_summary(), Style::default().fg(color)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items)
        .highlight_symbol("▸ ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, list_area, &mut app.list_state);

    // Scrollbar
    let visible = list_area.height as usize;
    if app.display_rows.len() > visible {
        let scroll_offset = app.list_state.offset();
        let mut scrollbar_state = ScrollbarState::new(app.display_rows.len().saturating_sub(visible))
            .position(scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        f.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
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

    if app.has_lazygit {
        append_key_hint(&mut spans, "l", "azygit");
    }

    // Always-present keys
    append_key_hint(&mut spans, "o", "rder");
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
