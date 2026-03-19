use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme::{ACTIVE_GREEN, CLAUDE_ORANGE, INACTIVE_GRAY};
use crate::data::types::Session;

pub fn render_sessions(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected: usize,
    username: &str,
    hostname: &str,
    scroll_offset: usize,
) {
    let is_wide = area.width >= 100;

    let mut lines: Vec<Line> = Vec::new();
    let mut row_starts: Vec<usize> = Vec::new(); // line index where each session starts

    for (i, session) in sessions.iter().enumerate() {
        row_starts.push(lines.len());
        let is_selected = i == selected;
        let base_style = if is_selected {
            Style::default()
                .fg(CLAUDE_ORANGE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Path line: "▶ user@host:/path" or "  user@host:/path"
        let marker = if is_selected { "▶ " } else { "  " };
        let path_line = Line::from(vec![Span::styled(
            format!("{}{}@{}:{}", marker, username, hostname, session.project_path),
            base_style,
        )]);
        lines.push(path_line);

        // Info content
        let display_name = session
            .project_path
            .rsplit('/')
            .next()
            .unwrap_or(&session.project_path);

        let active_span = if session.is_active {
            Span::styled(
                " ● ACTIVE",
                Style::default()
                    .fg(ACTIVE_GREEN)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" ○", Style::default().fg(INACTIVE_GRAY))
        };

        let tokens_str = format!("  {} tok", format_tokens(session.total_usage.total_output()));

        let model_str = if session.last_model.is_empty() {
            String::new()
        } else if session.has_thinking {
            format!("  {} / thinking", session.last_model)
        } else {
            format!("  {}", session.last_model)
        };

        if is_wide {
            // Single info line
            let mut spans = vec![
                Span::styled(format!("  {}", display_name), base_style),
                active_span,
                Span::styled(tokens_str, base_style),
            ];
            if !model_str.is_empty() {
                spans.push(Span::styled(model_str, Style::default().fg(INACTIVE_GRAY)));
            }
            lines.push(Line::from(spans));
        } else {
            // Two info lines
            let info_line1 = Line::from(vec![
                Span::styled(format!("  {}", display_name), base_style),
                active_span,
                Span::styled(tokens_str, base_style),
            ]);
            lines.push(info_line1);

            if !model_str.is_empty() {
                let info_line2 = Line::from(vec![Span::styled(
                    format!("  {}", model_str.trim()),
                    Style::default().fg(INACTIVE_GRAY),
                )]);
                lines.push(info_line2);
            }
        }

        // Empty line between sessions
        if i + 1 < sessions.len() {
            lines.push(Line::from(""));
        }
    }

    // Calculate scroll based on selected session
    let visible_height = area.height as usize;
    let selected_start = row_starts.get(selected).copied().unwrap_or(0);
    let lines_per_session = if is_wide { 3 } else { 4 }; // approximate
    let selected_end = selected_start + lines_per_session;

    let actual_offset = if selected_start < scroll_offset {
        selected_start
    } else if selected_end > scroll_offset + visible_height {
        selected_end.saturating_sub(visible_height)
    } else {
        scroll_offset
    };

    let paragraph = Paragraph::new(lines).scroll((actual_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
