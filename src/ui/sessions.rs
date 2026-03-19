use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Cell, Row, Table, Block, Borders};
use ratatui::Frame;

use super::theme::{BG_ACTIVE_ROW, BG_MAIN, BLUE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE, YELLOW};
use crate::data::types::{Session, SessionStatus};

pub fn render_sessions(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected: usize,
    scroll_offset: usize,
) {
    let header_cells = vec![
        Cell::from("PID").style(Style::default().fg(TEXT_DIM)),
        Cell::from("PROJECT").style(Style::default().fg(TEXT_DIM)),
        Cell::from("MODEL").style(Style::default().fg(TEXT_DIM)),
        Cell::from("STATUS").style(Style::default().fg(TEXT_DIM)),
        Cell::from("CTX USED / MAX").style(Style::default().fg(TEXT_DIM)),
        Cell::from("CACHE").style(Style::default().fg(TEXT_DIM)),
        Cell::from("COST").style(Style::default().fg(TEXT_DIM)),
        Cell::from("DURATION").style(Style::default().fg(TEXT_DIM)),
    ];
    let header = Row::new(header_cells)
        .style(Style::default().bg(BG_MAIN))
        .height(1)
        .bottom_margin(1);

    let rows = sessions.iter().enumerate().map(|(i, s)| {
        let is_selected = i == selected;
        let bg_color = if is_selected { BG_ACTIVE_ROW } else { BG_MAIN };
        
        let path_str = s.project_path.rsplit('/').next().unwrap_or(&s.project_path).to_string();
        
        let pid_str = s.pid.map_or("-".to_string(), |p| p.to_string());
        
        let status_span = match s.status {
            SessionStatus::Running => Span::styled("● running", Style::default().fg(GREEN)),
            SessionStatus::Waiting => Span::styled("⏸ waiting", Style::default().fg(YELLOW)),
            SessionStatus::Idle => Span::styled("○ idle", Style::default().fg(TEXT_IDLE)),
            SessionStatus::Error => Span::styled("✕ error", Style::default().fg(RED)),
        };

        let mut model_str = s.last_model.clone();
        if model_str.starts_with("claude-") {
            model_str = model_str.replace("claude-", "");
        }
        
        let max_ctx = if model_str.contains("opus") { 1_000_000 } else { 200_000 };
        let tokens = s.total_usage.total_input_all();
        let ctx_ratio = (tokens as f64 / max_ctx as f64).min(1.0);
        let ctx_color = if ctx_ratio > 0.9 { RED } else if ctx_ratio > 0.7 { ORANGE } else { BLUE };
        
        let bar_len: usize = 10;
        let filled_len = (ctx_ratio * bar_len as f64).round() as usize;
        let bar_str = format!("[{}{}]", "█".repeat(filled_len), "░".repeat(bar_len.saturating_sub(filled_len)));
        
        let ctx_str = format!("{} / {}k {} {:.0}%", 
            format_tokens(tokens), max_ctx / 1000, bar_str, ctx_ratio * 100.0);

        let hit_rate = s.total_usage.hit_rate();
        let hit_color = if hit_rate >= 60.0 { GREEN } else if hit_rate >= 30.0 { YELLOW } else { RED };

        let duration_secs = (s.last_activity - s.first_activity).num_seconds().max(0);
        let hours = duration_secs / 3600;
        let mins = (duration_secs % 3600) / 60;
        let duration_str = format!("{:02}:{:02}h", hours, mins);

        let cells = vec![
            Cell::from(Span::styled(pid_str, Style::default().fg(TEXT_HIGHLIGHT))),
            Cell::from(Span::styled(path_str, Style::default().fg(BLUE))),
            Cell::from(model_str),
            Cell::from(status_span),
            Cell::from(Span::styled(ctx_str, Style::default().fg(ctx_color))),
            Cell::from(Span::styled(format!("{:.0}%", hit_rate), Style::default().fg(hit_color))),
            Cell::from(format!("${:.2}", s.total_cost)),
            Cell::from(duration_str),
        ];

        let mut row_style = Style::default().bg(bg_color);
        if s.status == SessionStatus::Idle || s.status == SessionStatus::Error {
            row_style = row_style.fg(TEXT_IDLE);
        }
        
        Row::new(cells).style(row_style).height(1)
    });

    let widths = [
        Constraint::Length(6),  // PID
        Constraint::Length(18), // PROJECT
        Constraint::Length(12), // MODEL
        Constraint::Length(10), // STATUS
        Constraint::Length(25), // CTX USED/MAX
        Constraint::Length(6),  // CACHE
        Constraint::Length(8),  // COST
        Constraint::Length(9),  // DURATION
    ];

    // Scroll state management is required for Table if we want smooth scrolling.
    // For now we just use a basic offset if needed, or rely on ratatui's TableScroll state.
    // Since we don't have ratatui's stateful table setup here smoothly, we can manually slice.
    
    let visible_rows = area.height.saturating_sub(2) as usize;
    let actual_offset = if selected >= scroll_offset + visible_rows {
        selected - visible_rows + 1
    } else if selected < scroll_offset {
        selected
    } else {
        scroll_offset
    };

    let table = Table::new(rows.skip(actual_offset).take(visible_rows).collect::<Vec<_>>(), widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .column_spacing(1);

    frame.render_widget(table, area);
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
