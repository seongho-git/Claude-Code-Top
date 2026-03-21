use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use chrono::Local;

use super::confirm::render_confirm_dialog;
use super::details::render_details;
use super::header::render_header;
use super::threads::render_threads;
use super::theme::{BG_BORDER, BG_MAIN, BG_STATUS, BLUE, CLAUDE_ORANGE, GREEN, RED, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE, YELLOW};
use crate::app::{App, AppMode};
use crate::data::types::Thread;

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(BG_MAIN)),
        size,
    );

    // Build title with plan on left and current time on right
    let now_dt = Local::now();
    let tz_abbr = now_dt.format("%Z").to_string();
    let now = format!(
        "{} ({}) ",
        now_dt.format("%a %d %b %Y  %H:%M"),
        tz_abbr,
    );
    let left_title = format!(" Claude-Code-Top  [{}]", app.plan.label());
    let right_title = now;
    let available_width = (size.width as usize).saturating_sub(4);
    let padding_width = available_width.saturating_sub(left_title.len() + right_title.len());
    let full_title = format!("{}{}{}", left_title, " ".repeat(padding_width), right_title);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLAUDE_ORANGE))
        .title(Span::styled(
            full_title,
            Style::default().fg(CLAUDE_ORANGE),
        ));
    let inner = outer_block.inner(size);
    frame.render_widget(outer_block, size);

    let wide = inner.width >= 100;

    // Panel heights
    let header_height: u16 = 3; // 1 line per usage bar
    let spacer_height: u16 = 1;
    let local_header_height: u16 = 1;
    let status_height: u16 = 1;
    let details_height: u16 = 10;
    let threads_min: u16 = 3;
    let messages_max: u16 = 10;

    // Fixed overhead (always shown)
    let always_used = header_height + spacer_height + local_header_height + status_height; // 6
    let avail = inner.height.saturating_sub(always_used);

    // Extreme: not enough for even minimal threads
    if avail < threads_min {
        // Show only usage bars + status
        let minimal = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Min(0),
                Constraint::Length(status_height),
            ])
            .split(inner);
        render_header(frame, minimal[0], app, wide);
        render_status_bar(frame, minimal[2], app);
        render_confirm_overlay(frame, size, app);
        return;
    }

    let selected_thread = app.threads.get(app.selected);
    let threads_needed = (app.threads.len() as u16 + 2).max(threads_min);
    let msg_count = selected_thread
        .map(|t| t.recent_commands.len().min(messages_max as usize))
        .unwrap_or(0) as u16;

    // Degradation: determine what fits
    // Priority: threads > details > messages
    let (threads_h, msg_h, details_h);

    let with_details = threads_needed.max(threads_min) + details_height;
    if avail >= with_details {
        // Details fit — messages get leftover (max 10)
        details_h = details_height;
        let leftover = avail - with_details;
        msg_h = leftover.min(messages_max).min(msg_count);
        threads_h = avail - details_h - msg_h;
    } else if avail >= threads_min + details_height {
        // Squeeze threads to fit details, no messages
        details_h = details_height;
        msg_h = 0;
        threads_h = avail - details_h;
    } else {
        // No details, no messages — threads get everything
        details_h = 0;
        msg_h = 0;
        threads_h = avail;
    }

    // Build constraints
    let mut constraints = vec![
        Constraint::Length(header_height),
        Constraint::Length(spacer_height),
        Constraint::Length(local_header_height),
        Constraint::Length(threads_h),
    ];
    if msg_h > 0 {
        constraints.push(Constraint::Length(msg_h));
    }
    if details_h > 0 {
        constraints.push(Constraint::Length(details_h));
    }
    constraints.push(Constraint::Length(status_height));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut ci = 0;

    // Header
    render_header(frame, chunks[ci], app, wide);
    ci += 1;

    // Spacer
    ci += 1;

    // LOCAL THREADS label
    let local_header = Paragraph::new(Line::from(vec![
        Span::styled(
            " LOCAL THREADS ",
            Style::default().fg(BLUE).bg(BG_BORDER),
        ),
        Span::styled(
            format!("  {} active ", app.active_threads),
            Style::default().fg(GREEN),
        ),
    ]));
    frame.render_widget(local_header, chunks[ci]);
    ci += 1;

    // Thread list
    render_threads(
        frame,
        chunks[ci],
        &app.threads,
        app.selected,
        app.scroll_offset,
        app.sort_column,
    );
    ci += 1;

    // Recent messages (above detail panel)
    if msg_h > 0 {
        if let Some(thread) = selected_thread {
            render_messages(frame, chunks[ci], thread);
        }
        ci += 1;
    }

    // Detail panel
    if details_h > 0 {
        let detail_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[ci]);

        render_details(
            frame,
            detail_cols[0],
            detail_cols[1],
            selected_thread,
            app,
        );
        ci += 1;
    }

    // Status bar
    render_status_bar(frame, chunks[ci], app);

    // Confirm dialog overlay
    render_confirm_overlay(frame, size, app);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let live_indicator = if app.threads.iter().any(|t| t.is_active) {
        Span::styled(" ● live ", Style::default().fg(GREEN))
    } else {
        Span::styled(" ○ idle ", Style::default().fg(RED))
    };

    let sort_label = app.sort_column.label();
    let usage_age = format!("  usage: {}", app.usage_data.age_str());
    let status_text = Line::from(vec![
        Span::styled(" ←→:sort ", Style::default().fg(YELLOW)),
        Span::styled(
            format!("[{}] ", sort_label),
            Style::default().fg(CLAUDE_ORANGE),
        ),
        Span::styled(
            "F5:refresh  u:usage  d:del  q:quit  ",
            Style::default().fg(TEXT_IDLE),
        ),
        live_indicator,
        Span::styled(usage_age, Style::default().fg(TEXT_IDLE)),
    ]);
    let status_bar = Paragraph::new(status_text).style(Style::default().bg(BG_STATUS));
    frame.render_widget(status_bar, area);
}

fn render_confirm_overlay(frame: &mut Frame, size: ratatui::layout::Rect, app: &App) {
    if let AppMode::ConfirmDelete { index } = app.mode {
        let thread_name = app
            .threads
            .get(index)
            .map(|t| t.folder_name.as_str())
            .unwrap_or("unknown");
        render_confirm_dialog(frame, size, thread_name);
    }
}

fn render_messages(frame: &mut Frame, area: ratatui::layout::Rect, thread: &Thread) {
    if area.height == 0 {
        return;
    }
    let max_w = (area.width as usize).saturating_sub(4);
    let avail_lines = area.height as usize;

    let msgs: Vec<&String> = thread.recent_commands.iter().rev().take(avail_lines).collect();
    let mut lines = Vec::new();

    for (i, msg) in msgs.iter().rev().enumerate() {
        let is_last = i == msgs.len() - 1;
        let prefix = if is_last { " › " } else { "   " };
        let prefix_color = if is_last { BLUE } else { TEXT_DIM };

        let display = if msg.chars().count() > max_w {
            let truncated: String = msg.chars().take(max_w.saturating_sub(1)).collect();
            format!("{}…", truncated)
        } else {
            msg.to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(prefix_color)),
            Span::styled(
                display,
                Style::default().fg(if is_last { TEXT_HIGHLIGHT } else { TEXT_IDLE }),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
