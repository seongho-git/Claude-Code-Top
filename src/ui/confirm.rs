use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::{BG_BORDER, BG_PANEL, CLAUDE_ORANGE, TEXT_HIGHLIGHT, TEXT_IDLE};

pub fn render_confirm_dialog(frame: &mut Frame, area: Rect, session_name: &str) {
    let popup = centered_rect(60, 7, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .style(Style::default().bg(BG_PANEL))
        .title(Span::styled(
            " Confirm Delete ",
            Style::default().fg(CLAUDE_ORANGE),
        ));

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Forget session ", Style::default().fg(TEXT_IDLE)),
            Span::styled(
                session_name.to_string(),
                Style::default()
                    .fg(TEXT_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(TEXT_IDLE)),
        ]),
        Line::from(vec![
            Span::styled("  Press F to confirm", Style::default().fg(CLAUDE_ORANGE)),
            Span::styled(" · any other key cancels", Style::default().fg(TEXT_IDLE)),
        ]),
    ];

    frame.render_widget(Paragraph::new(text).block(block), popup);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
