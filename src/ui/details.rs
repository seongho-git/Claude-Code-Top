use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use super::theme::{
    BG_BORDER, BG_MAIN, BG_PANEL, BLUE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE,
    YELLOW,
};
use crate::data::types::Session;

pub fn render_details(
    frame: &mut Frame,
    left_area: Rect,
    right_area: Rect,
    session: Option<&Session>,
) {
    if let Some(s) = session {
        let model = if s.last_model.is_empty() {
            "-"
        } else {
            &s.last_model
        };
        let duration_secs = (s.last_activity - s.first_activity).num_seconds().max(0);
        let meta_lines = vec![
            Line::from(vec![
                Span::styled("PID ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    s.pid.map_or("-".to_string(), |p| p.to_string()),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
                Span::styled("   MODEL ", Style::default().fg(TEXT_DIM)),
                Span::styled(model, Style::default().fg(BLUE)),
            ]),
            Line::from(vec![
                Span::styled("EFFORT ", Style::default().fg(TEXT_DIM)),
                Span::styled(s.effort.label(), Style::default().fg(YELLOW)),
                Span::styled("   DURATION ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format_duration(duration_secs),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("PROJECT ", Style::default().fg(TEXT_DIM)),
                Span::styled(&s.project_path, Style::default().fg(TEXT_IDLE)),
            ]),
            Line::from(vec![
                Span::styled("LAST ACTIVE ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    s.last_activity.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(meta_lines).block(panel_block(" Session Overview ")),
            left_area,
        );

        let usage = &s.total_usage;
        let hit_rate = usage.hit_rate();
        let ctx_ratio = if model.contains("opus") {
            usage.total_input_all() as f64 / 1_000_000.0
        } else {
            usage.total_input_all() as f64 / 200_000.0
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(3),
                Constraint::Min(5),
            ])
            .split(right_area);

        let cost_lines = vec![
            Line::from(vec![
                Span::styled("Cost ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format!("${:.2}", s.total_cost),
                    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
                ),
                Span::styled("   Saved ", Style::default().fg(TEXT_DIM)),
                Span::styled(format!("${:.2}", s.saved_cost), Style::default().fg(GREEN)),
            ]),
            Line::from(vec![
                Span::styled("Input ", Style::default().fg(TEXT_DIM)),
                Span::raw(format_tokens(usage.input_tokens)),
                Span::styled("   Output ", Style::default().fg(TEXT_DIM)),
                Span::raw(format_tokens(usage.output_tokens)),
            ]),
            Line::from(vec![
                Span::styled("Cache read ", Style::default().fg(TEXT_DIM)),
                Span::raw(format_tokens(usage.cache_read_input_tokens)),
                Span::styled("   Cache write ", Style::default().fg(TEXT_DIM)),
                Span::raw(format_tokens(usage.cache_creation_input_tokens)),
            ]),
            Line::from(vec![
                Span::styled("Burn rate ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format!("{:.0} out tok/min", s.burn_rate),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(cost_lines).block(panel_block(" Usage & Cost ")),
            chunks[0],
        );

        frame.render_widget(
            Gauge::default()
                .block(panel_block(" Cache hit rate "))
                .gauge_style(
                    Style::default()
                        .fg(if hit_rate >= 60.0 {
                            GREEN
                        } else if hit_rate >= 30.0 {
                            YELLOW
                        } else {
                            RED
                        })
                        .bg(BG_MAIN),
                )
                .ratio((hit_rate / 100.0).clamp(0.0, 1.0))
                .label(format!("{:.0}%", hit_rate))
                .use_unicode(true),
            chunks[1],
        );

        let ctx_pct = ctx_ratio.clamp(0.0, 1.0);
        let ctx_color = if ctx_pct >= 0.9 {
            RED
        } else if ctx_pct >= 0.7 {
            ORANGE
        } else {
            BLUE
        };
        let detail_lines = vec![
            Line::from(vec![
                Span::styled("Session total tokens ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format_tokens(usage.total_tokens()),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Context used ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format!("{:.0}% of model window", ctx_pct * 100.0),
                    Style::default().fg(ctx_color),
                ),
            ]),
            Line::from(vec![
                Span::styled("Thinking mode ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    if s.has_thinking {
                        "enabled"
                    } else {
                        "not observed"
                    },
                    Style::default().fg(YELLOW),
                ),
            ]),
            Line::from(vec![
                Span::styled("Files ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    format!("{} jsonl logs", s.jsonl_files.len()),
                    Style::default().fg(TEXT_HIGHLIGHT),
                ),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(detail_lines).block(panel_block(" Session Detail ")),
            chunks[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new("No session selected").block(panel_block(" Session Overview ")),
            left_area,
        );
        frame.render_widget(
            Paragraph::new("No session selected").block(panel_block(" Session Detail ")),
            right_area,
        );
    }
}

fn panel_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .style(Style::default().bg(BG_PANEL))
        .title(Span::styled(
            title.to_string(),
            Style::default().fg(TEXT_HIGHLIGHT),
        ))
}
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
fn format_duration(secs: i64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    format!("{:02}:{:02}", hours, mins)
}
