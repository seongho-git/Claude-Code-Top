use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::confirm::render_confirm_dialog;
use super::details::render_details;
use super::header::render_header;
use super::sessions::render_sessions;
use super::sparkline::render_sparkline;
use super::theme::{BG_BORDER, BG_MAIN, BG_STATUS, GREEN, TEXT_HIGHLIGHT, TEXT_IDLE};
use crate::app::{App, AppMode};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG_MAIN)), size);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .title(Span::styled(
            " Claude-Code-Top ",
            Style::default().fg(TEXT_HIGHLIGHT),
        ));
    let inner = outer_block.inner(size);
    frame.render_widget(outer_block, size);

    if inner.height < 18 || inner.width < 72 {
        return;
    }

    let constraints = if inner.height >= 28 {
        vec![
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(12),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    render_header(frame, chunks[0], app);
    render_sessions(
        frame,
        chunks[1],
        &app.sessions,
        app.selected,
        app.scroll_offset,
    );

    let details_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    render_details(
        frame,
        details_chunks[0],
        details_chunks[1],
        app.sessions.get(app.selected),
    );
    render_sparkline(frame, chunks[3], &app.sparkline_data);

    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled("↑↓/jk move", Style::default().fg(TEXT_IDLE)),
        Span::raw("  "),
        Span::styled("s sort", Style::default().fg(TEXT_IDLE)),
        Span::raw("  "),
        Span::styled("r refresh", Style::default().fg(TEXT_IDLE)),
        Span::raw("  "),
        Span::styled("Delete confirm", Style::default().fg(TEXT_IDLE)),
        Span::raw("  "),
        Span::styled("Ctrl+C / Ctrl+D exit", Style::default().fg(GREEN)),
    ]))
    .style(Style::default().bg(BG_STATUS));
    frame.render_widget(status_bar, chunks[4]);

    if let AppMode::ConfirmDelete { index } = app.mode {
        let session_name = app
            .sessions
            .get(index)
            .map(|s| s.folder_name.as_str())
            .unwrap_or("unknown");
        render_confirm_dialog(frame, size, session_name);
    }
}
