use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use super::theme::{
    BG_ACTIVE_ROW, BG_BORDER, BG_MAIN, BLUE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT,
    TEXT_IDLE, YELLOW,
};
use crate::data::types::{Session, SessionStatus};

pub fn render_sessions(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected: usize,
    scroll_offset: usize,
) {
    let widths = session_widths(area.width);
    let header = Row::new(vec![
        "PID", "PROJECT", "MODEL", "EFFORT", "STATUS", "CTX", "CACHE", "COST", "DURATION",
    ])
    .style(
        Style::default()
            .fg(TEXT_DIM)
            .bg(BG_MAIN)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let visible_rows = area.height.saturating_sub(3) as usize;
    let actual_offset = if selected >= scroll_offset + visible_rows && visible_rows > 0 {
        selected - visible_rows + 1
    } else if selected < scroll_offset {
        selected
    } else {
        scroll_offset
    };

    let rows = sessions
        .iter()
        .skip(actual_offset)
        .take(visible_rows)
        .enumerate()
        .map(|(offset, s)| {
            let i = actual_offset + offset;
            let bg_color = if i == selected {
                BG_ACTIVE_ROW
            } else {
                BG_MAIN
            };
            let project = truncate_middle(
                s.project_path.rsplit('/').next().unwrap_or(&s.project_path),
                18,
            );
            let model = display_model(&s.last_model);
            let pid_str = s.pid.map_or("-".to_string(), |p| p.to_string());
            let status_span = match s.status {
                SessionStatus::Running => Span::styled("running", Style::default().fg(GREEN)),
                SessionStatus::Waiting => Span::styled("waiting", Style::default().fg(YELLOW)),
                SessionStatus::Idle => Span::styled("idle", Style::default().fg(TEXT_IDLE)),
                SessionStatus::Error => Span::styled("error", Style::default().fg(RED)),
            };
            let tokens = s.total_usage.total_input_all();
            let max_ctx = if model.contains("opus") {
                1_000_000
            } else {
                200_000
            };
            let ctx_ratio = (tokens as f64 / max_ctx as f64).min(1.0);
            let ctx_color = if ctx_ratio > 0.9 {
                RED
            } else if ctx_ratio > 0.7 {
                ORANGE
            } else {
                BLUE
            };
            let hit_rate = s.total_usage.hit_rate();
            let hit_color = if hit_rate >= 60.0 {
                GREEN
            } else if hit_rate >= 30.0 {
                YELLOW
            } else {
                RED
            };
            let duration_secs = (s.last_activity - s.first_activity).num_seconds().max(0);
            let duration_str = format_duration(duration_secs);

            Row::new(vec![
                Cell::from(Span::styled(pid_str, Style::default().fg(TEXT_HIGHLIGHT))),
                Cell::from(Span::styled(project, Style::default().fg(BLUE))),
                Cell::from(Span::styled(model, Style::default().fg(TEXT_HIGHLIGHT))),
                Cell::from(Span::styled(s.effort.label(), Style::default().fg(YELLOW))),
                Cell::from(status_span),
                Cell::from(Span::styled(
                    format!("{:.0}% {}", ctx_ratio * 100.0, format_tokens(tokens)),
                    Style::default().fg(ctx_color),
                )),
                Cell::from(Span::styled(
                    format!("{:.0}%", hit_rate),
                    Style::default().fg(hit_color),
                )),
                Cell::from(Span::styled(
                    format!("${:.2}", s.total_cost),
                    Style::default().fg(ORANGE),
                )),
                Cell::from(Span::styled(
                    duration_str,
                    Style::default().fg(TEXT_HIGHLIGHT),
                )),
            ])
            .style(Style::default().bg(bg_color))
        });

    let table = Table::new(rows.collect::<Vec<_>>(), widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BG_BORDER))
                .title(" Sessions "),
        )
        .column_spacing(1);
    frame.render_widget(table, area);
}

fn session_widths(width: u16) -> Vec<Constraint> {
    if width >= 120 {
        vec![
            Constraint::Length(6),
            Constraint::Min(18),
            Constraint::Length(18),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ]
    } else if width >= 96 {
        vec![
            Constraint::Length(5),
            Constraint::Min(14),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(8),
        ]
    } else {
        vec![
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(7),
        ]
    }
}

fn display_model(model: &str) -> String {
    if model.is_empty() {
        "-".to_string()
    } else {
        model.replace("claude-", "")
    }
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
fn format_duration(secs: i64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    if hours >= 100 {
        format!("{}h", hours)
    } else {
        format!("{:02}:{:02}", hours, mins)
    }
}
fn truncate_middle(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let left = max.saturating_sub(1) / 2;
    let right = max.saturating_sub(left + 1);
    let start: String = value.chars().take(left).collect();
    let end: String = value
        .chars()
        .rev()
        .take(right)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}…{}", start, end)
}
