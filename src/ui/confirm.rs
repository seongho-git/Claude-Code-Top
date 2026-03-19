use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::CLAUDE_ORANGE;

pub fn render_confirm_dialog(frame: &mut Frame, area: Rect, session_name: &str) {
    let popup = centered_rect(50, 5, area);

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
        Line::from(format!("  Delete session '{}'?", session_name)),
        Line::from(Span::styled(
            "  [y/N]",
            Style::default().add_modifier(Modifier::BOLD),
        )),
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
