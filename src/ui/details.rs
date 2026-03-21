use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::theme::{BG_BORDER, BLUE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE, YELLOW};
use super::{effort_display, format_tokens, shorten_model};
use crate::app::App;
use crate::data::types::Thread;

pub fn render_details(
    frame: &mut Frame,
    left_area: Rect,
    right_area: Rect,
    thread: Option<&Thread>,
    app: &App,
) {
    if let Some(s) = thread {
        render_left_panel(frame, left_area, s, app);
        render_right_panel(frame, right_area, s, app);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BG_BORDER));
        frame.render_widget(
            Paragraph::new("No thread selected").block(block.clone()),
            left_area,
        );
        frame.render_widget(
            Paragraph::new("No thread selected").block(block),
            right_area,
        );
    }
}

fn render_left_panel(frame: &mut Frame, area: Rect, s: &Thread, app: &App) {
    let mut lines = vec![];

    let display_name = s
        .project_path
        .rsplit('/')
        .next()
        .unwrap_or(&s.project_path);
    let status_type = if s.is_active { "local" } else { "offline" };
    let pid_str = s.pid.map_or("-".to_string(), |p| p.to_string());

    // Thread header
    lines.push(Line::from(vec![Span::styled(
        format!(" PID {}  ~{}  [{}]", pid_str, display_name, status_type),
        Style::default().fg(BLUE).add_modifier(Modifier::BOLD),
    )]));

    // Model + Effort (aligned: label 11 chars + value 12 chars = 23 | second col)
    let model_short = if s.last_model.is_empty() {
        "-".to_string()
    } else {
        shorten_model(&s.last_model)
    };
    let effort_str = effort_display(&s.last_effort);
    let effort_color = match effort_str {
        "Max" => ORANGE,
        "High" => YELLOW,
        "Low" => TEXT_DIM,
        _ => TEXT_HIGHLIGHT,
    };
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<10}", "Model:"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:<12}", model_short), Style::default().fg(TEXT_HIGHLIGHT)),
        Span::styled("Effort: ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            effort_str,
            Style::default()
                .fg(effort_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Duration + Burn rate (aligned)
    let duration_secs = (s.last_activity - s.first_activity).num_seconds().max(0);
    let hours = duration_secs / 3600;
    let mins = (duration_secs % 3600) / 60;
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<10}", "Duration:"), Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:<12}", format!("{:02}:{:02}h", hours, mins)),
            Style::default().fg(TEXT_HIGHLIGHT),
        ),
        Span::styled("Burn:   ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:.0} tok/m", s.burn_rate),
            Style::default().fg(TEXT_HIGHLIGHT),
        ),
    ]));

    // Blank line
    lines.push(Line::from(""));

    // Tokens: in + out (aligned, hit moved to cost line)
    let usage = &s.total_usage;
    let hit_rate = usage.hit_rate();
    let hit_color = if hit_rate >= 60.0 {
        GREEN
    } else if hit_rate >= 30.0 {
        YELLOW
    } else {
        RED
    };
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<10}", "Tokens:"), Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:<12}", format!("in {:>7}", format_tokens(usage.input_tokens))),
            Style::default().fg(TEXT_IDLE),
        ),
        Span::styled(
            format!("out {:>7}", format_tokens(usage.output_tokens)),
            Style::default().fg(TEXT_IDLE),
        ),
    ]));

    // Cost + hit (aligned)
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<10}", "Cost:"), Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:<12}", format!("${:.4}", s.total_cost)),
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("hit {:.0}%", hit_rate),
            Style::default().fg(hit_color),
        ),
        Span::styled(
            format!(" (saved ${:.2})", s.saved_cost),
            Style::default().fg(GREEN),
        ),
    ]));

    // Messages remaining
    let total_5h_msgs = app.window_5h_messages;
    if app.usage_data.is_available() && app.usage_data.session_pct > 0.0 {
        let pct_used = app.usage_data.session_pct;
        let msgs_remaining = if pct_used < 100.0 {
            (total_5h_msgs as f64 * (100.0 - pct_used) / pct_used).round() as u64
        } else {
            0
        };
        let msg_color = if msgs_remaining < 5 {
            RED
        } else if msgs_remaining < 20 {
            ORANGE
        } else {
            GREEN
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<10}", "Remain:"), Style::default().fg(TEXT_DIM)),
            Span::styled(
                format!("~{} msgs", msgs_remaining),
                Style::default().fg(msg_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({:.0}% used, {} in 5h)", pct_used, total_5h_msgs),
                Style::default().fg(TEXT_IDLE),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<10}", "5h:"), Style::default().fg(TEXT_DIM)),
            Span::styled(
                format!("{} msgs sent", total_5h_msgs),
                Style::default().fg(TEXT_HIGHLIGHT),
            ),
        ]));
    }

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .title(Span::styled(
            " Thread Details",
            Style::default().fg(BLUE),
        ));
    frame.render_widget(Paragraph::new(lines).block(left_block), area);
}

fn render_right_panel(frame: &mut Frame, area: Rect, s: &Thread, app: &App) {
    let mut lines = vec![];
    let inner_w = (area.width as usize).saturating_sub(3);

    // Project path (truncated if long)
    let path_display = if s.project_path.len() > (area.width as usize).saturating_sub(9) {
        let truncated = &s.project_path
            [s.project_path.len().saturating_sub((area.width as usize).saturating_sub(12))..];
        format!("\u{2026}{}", truncated)
    } else {
        s.project_path.clone()
    };
    lines.push(Line::from(vec![
        Span::styled(" Path:  ", Style::default().fg(TEXT_DIM)),
        Span::styled(path_display, Style::default().fg(TEXT_IDLE)),
    ]));

    // First / Last activity
    let first_str = s.first_activity.format("%m-%d %H:%M").to_string();
    let last_str = s.last_activity.format("%m-%d %H:%M").to_string();
    lines.push(Line::from(vec![
        Span::styled(" First: ", Style::default().fg(TEXT_DIM)),
        Span::styled(&first_str, Style::default().fg(TEXT_IDLE)),
        Span::styled("  Last: ", Style::default().fg(TEXT_DIM)),
        Span::styled(&last_str, Style::default().fg(TEXT_IDLE)),
    ]));

    // Thread Model section
    if !s.per_model_usage.is_empty() {
        let tm_label = " Thread Model ";
        let tm_dash = inner_w.saturating_sub(tm_label.len());
        let tm_l = tm_dash / 2;
        let tm_r = tm_dash - tm_l;
        lines.push(Line::from(vec![
            Span::styled(format!(" {}", "\u{2500}".repeat(tm_l)), Style::default().fg(TEXT_DIM)),
            Span::styled(tm_label, Style::default().fg(TEXT_HIGHLIGHT)),
            Span::styled("\u{2500}".repeat(tm_r), Style::default().fg(TEXT_DIM)),
        ]));

        let total_toks = s.total_usage.total_input_all() + s.total_usage.output_tokens;
        let (mut opus_t, mut sonnet_t, mut haiku_t) = (0u64, 0u64, 0u64);
        for (m, u) in &s.per_model_usage {
            let t = u.total_input_all() + u.output_tokens;
            let lower = m.to_lowercase();
            if lower.contains("opus") {
                opus_t += t;
            } else if lower.contains("haiku") {
                haiku_t += t;
            } else {
                sonnet_t += t;
            }
        }

        let tier_list: Vec<(u64, ratatui::style::Color)> =
            [(opus_t, ORANGE), (sonnet_t, BLUE), (haiku_t, GREEN)]
                .iter()
                .filter(|(t, _)| *t > 0)
                .copied()
                .collect();

        // Distribution bar
        let bar_w = (area.width as usize).saturating_sub(6).max(5);
        let mut bar_spans = vec![Span::styled("  [", Style::default().fg(TEXT_DIM))];
        let mut chars_used = 0;
        for (idx, (tokens, color)) in tier_list.iter().enumerate() {
            let pct_f = if total_toks > 0 { *tokens as f64 / total_toks as f64 } else { 0.0 };
            let chars = if idx == tier_list.len() - 1 {
                bar_w.saturating_sub(chars_used)
            } else {
                (pct_f * bar_w as f64).round() as usize
            };
            chars_used += chars;
            if chars > 0 {
                bar_spans.push(Span::styled("\u{2588}".repeat(chars), Style::default().fg(*color)));
            }
        }
        bar_spans.push(Span::styled("]", Style::default().fg(TEXT_DIM)));
        lines.push(Line::from(bar_spans));

        // Tier percentages
        let pct = |t: u64| -> u64 {
            if total_toks > 0 { (t as f64 / total_toks as f64 * 100.0).round() as u64 } else { 0 }
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  opus {:>3}%", pct(opus_t)), Style::default().fg(ORANGE)),
            Span::styled(format!("  sonnet {:>3}%", pct(sonnet_t)), Style::default().fg(BLUE)),
            Span::styled(format!("  haiku {:>3}%", pct(haiku_t)), Style::default().fg(GREEN)),
        ]));
    }

    // All-Time section
    {
        let at_label = " All-Time ";
        let at_dash = inner_w.saturating_sub(at_label.len());
        let at_l = at_dash / 2;
        let at_r = at_dash - at_l;
        lines.push(Line::from(vec![
            Span::styled(format!(" {}", "\u{2500}".repeat(at_l)), Style::default().fg(TEXT_DIM)),
            Span::styled(at_label, Style::default().fg(TEXT_HIGHLIGHT)),
            Span::styled("\u{2500}".repeat(at_r), Style::default().fg(TEXT_DIM)),
        ]));

        let tier_colors = [ORANGE, BLUE, GREEN];
        let mut tier_data: [(u64, f64); 3] = [(0, 0.0); 3];
        for (model_name, tokens, cost) in app.lifetime_by_model.iter() {
            let lower = model_name.to_lowercase();
            let idx = if lower.contains("opus") { 0 } else if lower.contains("haiku") { 2 } else { 1 };
            tier_data[idx].0 += tokens;
            tier_data[idx].1 += cost;
        }

        // Line 1: Total + opus
        lines.push(Line::from(vec![
            Span::styled(format!(" Total {:>6}", format_tokens(app.total_tokens_all)), Style::default().fg(TEXT_HIGHLIGHT)),
            Span::styled(format!(" ${:<6.2}", app.total_cost_all), Style::default().fg(ORANGE)),
            Span::styled(format!(" opus {:>6}", format_tokens(tier_data[0].0)), Style::default().fg(tier_colors[0])),
            Span::styled(format!(" ${:.2}", tier_data[0].1), Style::default().fg(ORANGE)),
        ]));
        // Line 2: sonnet + haiku
        lines.push(Line::from(vec![
            Span::styled(format!(" sonnet {:>5}", format_tokens(tier_data[1].0)), Style::default().fg(tier_colors[1])),
            Span::styled(format!(" ${:<6.2}", tier_data[1].1), Style::default().fg(ORANGE)),
            Span::styled(format!(" haiku {:>5}", format_tokens(tier_data[2].0)), Style::default().fg(tier_colors[2])),
            Span::styled(format!(" ${:.2}", tier_data[2].1), Style::default().fg(ORANGE)),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_BORDER))
        .title(Span::styled(
            " Thread & Usage",
            Style::default().fg(BLUE),
        ));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

