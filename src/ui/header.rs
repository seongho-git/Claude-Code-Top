use ratatui::layout::{Constraint, Layout, Rect, Direction};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme::{GREEN, ORANGE, TEXT_IDLE, BG_MAIN};
use crate::app::App;

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let base_style = Style::default().bg(BG_MAIN);

    // 1. Daily Cost
    let daily_limit = app.plan.cost_limit() / 30.0;
    let cost_color = if app.daily_cost > daily_limit { ORANGE } else { GREEN };
    let daily_text = Line::from(vec![
        Span::styled("Daily cost: ", Style::default().fg(TEXT_IDLE)),
        Span::styled(format!("${:.2} / ${:.2}", app.daily_cost, daily_limit), Style::default().fg(cost_color).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(daily_text).style(base_style), chunks[0]);

    // 2. Monthly Cost
    let monthly_limit = app.plan.cost_limit();
    let m_color = if app.monthly_cost > monthly_limit { ORANGE } else { GREEN };
    let monthly_text = Line::from(vec![
        Span::styled("Monthly cost: ", Style::default().fg(TEXT_IDLE)),
        Span::styled(format!("${:.2} / ${:.2}", app.monthly_cost, monthly_limit), Style::default().fg(m_color).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(monthly_text).style(base_style), chunks[1]);

    // 3. Cache Saved Today
    let saved_text = Line::from(vec![
        Span::styled("Cache saved: ", Style::default().fg(TEXT_IDLE)),
        Span::styled(format!("${:.2}", app.daily_saved), Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(saved_text).style(base_style), chunks[2]);

    // 4. Sessions
    let sessions_text = Line::from(vec![
        Span::styled(format!("{} local  0 remote ", app.sessions.len()), Style::default().fg(TEXT_IDLE)),
        Span::styled(format!("● {} active", app.active_sessions), Style::default().fg(GREEN)),
    ]);
    frame.render_widget(Paragraph::new(sessions_text).style(base_style), chunks[3]);
}
