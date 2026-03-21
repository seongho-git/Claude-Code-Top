use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::{BG_MAIN, CLAUDE_ORANGE, TEXT_DIM, TEXT_HIGHLIGHT};

pub fn render_confirm_dialog(frame: &mut Frame, area: Rect, session_name: &str) {
    let popup = centered_rect(50, 7, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLAUDE_ORANGE))
        .title(Span::styled(
            " Confirm Delete ",
            Style::default()
                .fg(CLAUDE_ORANGE)
                .add_modifier(Modifier::BOLD),
        ));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Delete thread '{}'?", session_name),
            Style::default().fg(TEXT_HIGHLIGHT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " Enter ",
                Style::default()
                    .fg(BG_MAIN)
                    .bg(CLAUDE_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Confirm   ", Style::default().fg(TEXT_HIGHLIGHT)),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(BG_MAIN)
                    .bg(TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(TEXT_HIGHLIGHT)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, popup);
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
