use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use super::theme::{
    BG_BORDER, BG_MAIN, BG_PANEL, GREEN, ORANGE, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE, YELLOW,
};
use crate::app::App;

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let daily_limit = app.plan.cost_limit() / 30.0;
    let monthly_limit = app.plan.cost_limit();
    render_cost_card(frame, top[0], "Daily cost", app.daily_cost, daily_limit);
    render_cost_card(
        frame,
        top[1],
        "Monthly cost",
        app.monthly_cost,
        monthly_limit,
    );

    render_text_card(
        frame,
        bottom[0],
        "Sessions",
        vec![Line::from(vec![
            Span::styled(
                format!("{} total", app.sessions.len()),
                Style::default().fg(TEXT_HIGHLIGHT),
            ),
            Span::styled("  ·  ", Style::default().fg(TEXT_DIM)),
            Span::styled(
                format!("{} active", app.active_sessions),
                Style::default().fg(GREEN),
            ),
            Span::styled("  ·  ", Style::default().fg(TEXT_DIM)),
            Span::styled(app.plan.label(), Style::default().fg(YELLOW)),
        ])],
    );
    render_text_card(
        frame,
        bottom[1],
        "Cache saved this month",
        vec![Line::from(vec![
            Span::styled(
                format!("${:.2}", app.monthly_saved),
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  based on current calendar month",
                Style::default().fg(TEXT_IDLE),
            ),
        ])],
    );
}

fn render_cost_card(frame: &mut Frame, area: Rect, title: &str, value: f64, limit: f64) {
    let ratio = if limit > 0.0 {
        (value / limit).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let color = if ratio >= 0.9 { ORANGE } else { GREEN };
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .style(Style::default().bg(BG_PANEL))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(TEXT_DIM),
        ));
    frame.render_widget(block, area);

    let text = Line::from(vec![
        Span::styled(
            format!("${:.2}", value),
            Style::default()
                .fg(TEXT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" / ${:.2}", limit), Style::default().fg(TEXT_IDLE)),
    ]);
    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(BG_PANEL)),
        inner[0],
    );
    frame.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(color).bg(BG_MAIN))
            .ratio(ratio)
            .label(format!("{:.0}%", ratio * 100.0))
            .use_unicode(true),
        inner[1],
    );
}

fn render_text_card(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .style(Style::default().bg(BG_PANEL))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(TEXT_DIM),
        ));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
