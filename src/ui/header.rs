use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Gauge;
use ratatui::Frame;

use super::theme::{BAR_BG, CLAUDE_ORANGE};
use crate::data::types::PlanType;

pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    weekly_cost: f64,
    weekly_tokens: u64,
    plan: PlanType,
) {
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    // Cost bar
    let cost_limit = plan.cost_limit();
    let cost_ratio = if cost_limit > 0.0 {
        (weekly_cost / cost_limit).min(1.0)
    } else {
        0.0
    };
    let cost_pct = (cost_ratio * 100.0) as u16;

    let cost_label = format!(
        "  Weekly:  ${:.2} / ${:.2}  {}%",
        weekly_cost, cost_limit, cost_pct
    );

    let cost_gauge = Gauge::default()
        .gauge_style(Style::default().fg(CLAUDE_ORANGE).bg(BAR_BG))
        .ratio(cost_ratio)
        .label(Span::raw(cost_label));

    frame.render_widget(cost_gauge, chunks[0]);

    // Token bar
    let token_limit = plan.token_limit();
    let token_ratio = if token_limit > 0 {
        (weekly_tokens as f64 / token_limit as f64).min(1.0)
    } else {
        0.0
    };
    let token_pct = (token_ratio * 100.0) as u16;

    let token_label = format!(
        "  Tokens:  {} / {}  {}%",
        format_tokens(weekly_tokens),
        format_tokens(token_limit),
        token_pct
    );

    let token_gauge = Gauge::default()
        .gauge_style(Style::default().fg(CLAUDE_ORANGE).bg(BAR_BG))
        .ratio(token_ratio)
        .label(Span::raw(token_label));

    frame.render_widget(token_gauge, chunks[1]);
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
