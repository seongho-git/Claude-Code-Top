use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use super::confirm::render_confirm_dialog;
use super::header::render_header;
use super::sessions::render_sessions;
use super::theme::CLAUDE_ORANGE;
use crate::app::{App, AppMode};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Outer border
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLAUDE_ORANGE))
        .title(Span::styled(
            " cctop ",
            Style::default()
                .fg(CLAUDE_ORANGE)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            format!(" [{}]  Ctrl+C: quit ", app.plan.label()),
            Style::default().fg(CLAUDE_ORANGE),
        ));

    frame.render_widget(outer_block, size);

    // Inner area (inside border)
    let inner = Rect {
        x: size.x + 1,
        y: size.y + 1,
        width: size.width.saturating_sub(2),
        height: size.height.saturating_sub(2),
    };

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    // Split: header (2 lines) + separator label (1 line) + sessions list
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(inner);

    // Header: cost and token bars
    render_header(
        frame,
        chunks[0],
        app.weekly_cost,
        app.weekly_tokens,
        app.plan,
    );

    // Separator with navigation hints
    let sep_text = Span::styled(
        " Sessions                              ↑↓:navigate   q:delete",
        Style::default().fg(CLAUDE_ORANGE),
    );
    frame.render_widget(
        ratatui::widgets::Paragraph::new(ratatui::text::Line::from(sep_text)),
        chunks[1],
    );

    // Session list
    render_sessions(
        frame,
        chunks[2],
        &app.sessions,
        app.selected,
        &app.username,
        &app.hostname,
        app.scroll_offset,
    );

    // Confirm dialog overlay
    if let AppMode::ConfirmDelete { index } = app.mode {
        let session_name = app
            .sessions
            .get(index)
            .map(|s| {
                s.project_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&s.project_path)
            })
            .unwrap_or("unknown");
        render_confirm_dialog(frame, size, session_name);
    }
}
