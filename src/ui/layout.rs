use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::confirm::render_confirm_dialog;
use super::header::render_header;
use super::sessions::render_sessions;
use super::details::render_details;
use super::sparkline::render_sparkline;
use super::theme::{BG_BORDER, BG_MAIN, BG_STATUS, BLUE, CLAUDE_ORANGE, GREEN, RED, TEXT_IDLE};
use crate::app::{App, AppMode};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Base background
    frame.render_widget(Block::default().style(Style::default().bg(BG_MAIN)), size);

    // Title Bar border
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .title(Span::styled(
            " 🔴 🟡 🟢  claude-code-monitor ",
            Style::default().fg(CLAUDE_ORANGE).add_modifier(Modifier::BOLD),
        ));
    let inner = outer_block.inner(size);
    frame.render_widget(outer_block, size);

    if inner.height < 15 || inner.width < 50 {
        return; // Terminal too small
    }

    // Main vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // 0: Summary Header
            Constraint::Length(1), // 1: LOCAL SESSIONS Header
            Constraint::Min(5),    // 2: Sessions List
            Constraint::Length(12),// 3: Session Details Panel (Split)
            Constraint::Length(3), // 4: Sparkline
            Constraint::Length(1), // 5: Status Bar
        ])
        .split(inner);

    // 0. Summary Header
    render_header(frame, chunks[0], app);

    // 1. Local Sessions Header
    let local_header = Paragraph::new(Line::from(vec![
        Span::styled(" LOCAL SESSIONS ", Style::default().fg(BLUE).bg(BG_BORDER)),
        Span::styled(format!("  {} active ", app.active_sessions), Style::default().fg(GREEN)),
    ]));
    frame.render_widget(local_header, chunks[1]);

    // 2. Session List
    render_sessions(
        frame,
        chunks[2],
        &app.sessions,
        app.selected,
        app.scroll_offset,
    );

    // 3. Detail Panel (Split Left/Right)
    let details_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[3]);
        
    let selected_session = app.sessions.get(app.selected);
    render_details(frame, details_chunks[0], details_chunks[1], selected_session);

    // 4. Sparkline
    render_sparkline(frame, chunks[4], &app.sparkline_data);

    // 5. Status Bar
    let live_indicator = if app.sessions.iter().any(|s| s.is_active) {
        Span::styled(" ● live ", Style::default().fg(GREEN))
    } else {
        Span::styled(" ○ idle ", Style::default().fg(RED))
    };
    
    let status_text = Line::from(vec![
        Span::styled(" F1:help  F2:sort  F5:refresh  q:quit    ", Style::default().fg(TEXT_IDLE)),
        Span::styled("auto-refresh: 2s  ", Style::default().fg(TEXT_IDLE)),
        live_indicator,
    ]);
    let status_bar = Paragraph::new(status_text)
        .style(Style::default().bg(BG_STATUS));
    frame.render_widget(status_bar, chunks[5]);

    // Confirm dialog overlay
    if let AppMode::ConfirmDelete { index } = app.mode {
        let session_name = app
            .sessions
            .get(index)
            .map(|s| s.folder_name.as_str())
            .unwrap_or("unknown");
        render_confirm_dialog(frame, size, session_name);
    }
}
