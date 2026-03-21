use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::format_tokens;
use super::theme::{BG_MAIN, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT};
use crate::app::App;

/// Render 3 single-line usage bars (session, weekly, extra).
pub fn render_header(frame: &mut Frame, area: Rect, app: &App, _wide: bool) {
    let base_style = Style::default().bg(BG_MAIN);
    let has_usage = app.usage_data.is_available();
    let is_stale = app.usage_data.is_stale();
    let marker = if !has_usage || is_stale { "~" } else { "" };
    let stale_tag = if is_stale && has_usage { " \u{26a0}" } else { "" };

    // Bar width = 25% of total width
    let bar_width = ((area.width as usize) / 4).max(5);

    // Session
    let session_pct = if has_usage {
        app.usage_data.session_pct
    } else {
        let limit = app.plan.token_limit();
        if limit > 0 { (app.window_5h_tokens as f64 / limit as f64 * 100.0).min(100.0) } else { 0.0 }
    };
    let tok_5h = format_tokens(app.window_5h_tokens);
    let msgs_5h = app.window_5h_messages;
    let session_info = if has_usage && !app.usage_data.session_reset.is_empty() {
        format!("Resets {}  ({} tok, {} msgs)", app.usage_data.session_reset, tok_5h, msgs_5h)
    } else if let Some(reset) = app.window_5h_reset {
        let now = chrono::Utc::now();
        if reset > now {
            let m = (reset - now).num_minutes();
            format!("~{}h{:02}m  ({} tok, {} msgs)", m / 60, m % 60, tok_5h, msgs_5h)
        } else {
            format!("reset available  ({} tok, {} msgs)", tok_5h, msgs_5h)
        }
    } else {
        "no recent activity".to_string()
    };

    // Weekly
    let weekly_pct = if has_usage {
        app.usage_data.weekly_pct
    } else {
        let limit = app.plan.cost_limit();
        if limit > 0.0 { (app.weekly_cost / limit * 100.0).min(100.0) } else { 0.0 }
    };
    let weekly_info = if has_usage && !app.usage_data.weekly_reset.is_empty() {
        format!("Resets {}", app.usage_data.weekly_reset)
    } else {
        format!("${:.2} / ${:.2}", app.weekly_cost, app.plan.cost_limit())
    };

    // Extra
    let extra_pct = if has_usage { app.usage_data.extra_pct } else { 0.0 };
    let extra_info = if has_usage {
        let spent = if !app.usage_data.extra_spent.is_empty() {
            format!("{}  ", app.usage_data.extra_spent)
        } else {
            String::new()
        };
        let reset = if !app.usage_data.extra_reset.is_empty() {
            format!("Resets {}", app.usage_data.extra_reset)
        } else {
            String::new()
        };
        format!("{}{}", spent, reset)
    } else {
        "n/a".to_string()
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    render_usage_row(frame, rows[0], base_style, "Session", marker, session_pct, &session_info, stale_tag, bar_width);
    render_usage_row(frame, rows[1], base_style, "Weekly", marker, weekly_pct, &weekly_info, stale_tag, bar_width);
    render_usage_row(frame, rows[2], base_style, "Extra", marker, extra_pct, &extra_info, stale_tag, bar_width);
}

/// Single-line usage row: " Session~  [████░░]  65%  Resets Mar 20  (1.2M tok) ⚠"
fn render_usage_row(
    frame: &mut Frame,
    area: Rect,
    base_style: Style,
    label: &str,
    marker: &str,
    pct: f64,
    info: &str,
    stale_tag: &str,
    bar_width: usize,
) {
    let color = usage_color(pct);
    let ratio = (pct / 100.0).clamp(0.0, 1.0);
    let bar = make_bar(ratio, bar_width);

    let mut spans = vec![
        Span::styled(
            format!(" {:<8}{}", label, marker),
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD),
        ),
        Span::styled(bar, Style::default().fg(color)),
        Span::styled(
            format!(" {:>3.0}%", pct),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}", info),
            Style::default().fg(TEXT_HIGHLIGHT),
        ),
    ];
    if !stale_tag.is_empty() {
        spans.push(Span::styled(
            stale_tag,
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)).style(base_style), area);
}

fn usage_color(pct: f64) -> ratatui::style::Color {
    if pct > 85.0 { RED } else if pct > 50.0 { ORANGE } else { GREEN }
}

fn make_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}
