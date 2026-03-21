use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use super::theme::{
    BG_ACTIVE_ROW, BG_MAIN, BLUE, CLAUDE_ORANGE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT,
    TEXT_IDLE, YELLOW,
};
use super::{effort_display, format_tokens, shorten_model};
use crate::app::SortColumn;
use crate::data::types::{context_max, Thread, ThreadStatus};

pub fn render_threads(
    frame: &mut Frame,
    area: Rect,
    threads: &[Thread],
    selected: usize,
    scroll_offset: usize,
    sort_column: SortColumn,
) {
    let w = area.width;

    // Fixed column widths
    let pid_w: u16 = 7;
    let status_w: u16 = 10;
    let model_w: u16 = 12;
    let effort_w: u16 = 6;
    let ctx_w: u16 = 14;
    let cache_w: u16 = 6;
    let cost_w: u16 = 9;
    let duration_w: u16 = 8;

    // DIRECTORY width: fixed based on max path length across all threads
    let dir_w = {
        let max_len = threads
            .iter()
            .map(|t| format_project_path(&t.project_path, 60).chars().count())
            .max()
            .unwrap_or(10) as u16;
        max_len.min(30).max(8)
    };

    // Responsive: hide from rightmost column first
    // Order of hiding: DURATION → COST → CACHE → CTX → EFFORT → MODEL → STATUS → DIRECTORY
    let base = pid_w + 1 + dir_w + 1 + 20; // PID + DIR + PROJECT(min=20)
    let show_dir = w >= base;
    let show_status = w >= base + status_w + 1;
    let show_model = w >= base + status_w + 1 + model_w + 1;
    let show_effort = w >= base + status_w + 1 + model_w + 1 + effort_w + 1;
    let show_ctx = w >= base + status_w + 1 + model_w + 1 + effort_w + 1 + ctx_w + 1;
    let show_cache = w >= base + status_w + 1 + model_w + 1 + effort_w + 1 + ctx_w + 1 + cache_w + 1;
    let show_cost = w >= base + status_w + 1 + model_w + 1 + effort_w + 1 + ctx_w + 1 + cache_w + 1 + cost_w + 1;
    let show_duration = w >= base + status_w + 1 + model_w + 1 + effort_w + 1 + ctx_w + 1 + cache_w + 1 + cost_w + 1 + duration_w + 1;

    // Calculate fixed width used by visible columns
    let mut fixed: u16 = pid_w + 1;
    if show_dir {
        fixed += dir_w + 1;
    }
    if show_status {
        fixed += status_w + 1;
    }
    if show_model {
        fixed += model_w + 1;
    }
    if show_effort {
        fixed += effort_w + 1;
    }
    if show_ctx {
        fixed += ctx_w + 1;
    }
    if show_cache {
        fixed += cache_w + 1;
    }
    if show_cost {
        fixed += cost_w + 1;
    }
    if show_duration {
        fixed += duration_w + 1;
    }

    let project_w = w.saturating_sub(fixed).max(20);

    // Header style: highlight sorted column in orange
    let col_style = |col: SortColumn| {
        if sort_column == col {
            Style::default()
                .fg(CLAUDE_ORANGE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_DIM)
        }
    };

    let mut header_cells = Vec::new();
    let mut widths = Vec::new();

    header_cells.push(Cell::from(center("PID", pid_w as usize)).style(col_style(SortColumn::Pid)));
    widths.push(Constraint::Length(pid_w));

    if show_dir {
        header_cells.push(Cell::from("DIRECTORY").style(col_style(SortColumn::Project)));
        widths.push(Constraint::Length(dir_w));
    }

    header_cells.push(Cell::from(center("PROJECT", project_w as usize)).style(col_style(SortColumn::Project)));
    widths.push(Constraint::Length(project_w));

    if show_status {
        header_cells.push(Cell::from(center("STATUS", status_w as usize)).style(col_style(SortColumn::Status)));
        widths.push(Constraint::Length(status_w));
    }
    if show_model {
        header_cells.push(Cell::from(center("MODEL", model_w as usize)).style(col_style(SortColumn::Model)));
        widths.push(Constraint::Length(model_w));
    }
    if show_effort {
        header_cells.push(Cell::from(center("EFFORT", effort_w as usize)).style(col_style(SortColumn::Effort)));
        widths.push(Constraint::Length(effort_w));
    }
    if show_ctx {
        header_cells.push(Cell::from(center("CTX", ctx_w as usize)).style(col_style(SortColumn::Ctx)));
        widths.push(Constraint::Length(ctx_w));
    }
    if show_cache {
        header_cells.push(Cell::from(center("CACHE", cache_w as usize)).style(col_style(SortColumn::Cache)));
        widths.push(Constraint::Length(cache_w));
    }
    if show_cost {
        header_cells.push(Cell::from(center("COST", cost_w as usize)).style(col_style(SortColumn::Cost)));
        widths.push(Constraint::Length(cost_w));
    }
    if show_duration {
        header_cells.push(Cell::from(center("DURATION", duration_w as usize)).style(col_style(SortColumn::Duration)));
        widths.push(Constraint::Length(duration_w));
    }

    let header = Row::new(header_cells)
        .style(Style::default().bg(BG_MAIN))
        .height(1)
        .bottom_margin(0);

    let rows = threads.iter().enumerate().map(|(i, t)| {
        let is_selected = i == selected;
        let bg_color = if is_selected { BG_ACTIVE_ROW } else { BG_MAIN };

        let dir_display = format_project_path(&t.project_path, dir_w as usize);

        // PROJECT column: first N words of last message (blue) + session id (gray)
        let session_short = if t.session_file.len() > 8 {
            &t.session_file[..8]
        } else {
            &t.session_file
        };
        let word_count = if project_w >= 30 { 5 } else { 2 };
        let last_cmd_words = t.recent_commands.last()
            .and_then(|s| {
                let trimmed = s.trim().replace(['\n', '\r'], " ");
                let words: Vec<&str> = trimmed.split_whitespace().take(word_count).collect();
                if words.is_empty() { None } else { Some(words.join(" ")) }
            });
        let project_display = if let Some(ref words) = last_cmd_words {
            let avail = (project_w as usize).saturating_sub(session_short.len() + 1);
            let w = truncate_str(words, avail);
            (w, session_short.to_string())
        } else {
            let dir_name = t.project_path.rsplit('/').next().unwrap_or(&t.project_path);
            let avail = (project_w as usize).saturating_sub(session_short.len() + 1);
            let d = truncate_str(dir_name, avail);
            (d, session_short.to_string())
        };

        let pid_str = t.pid.map_or("-".to_string(), |p| p.to_string());

        let (status_text, status_color) = match t.status {
            ThreadStatus::Running => ("● running", GREEN),
            ThreadStatus::Waiting => ("⏸ waiting", YELLOW),
            ThreadStatus::Idle    => ("○ idle",    TEXT_IDLE),
            ThreadStatus::Error   => ("✕ error",   RED),
        };
        let status_span = Span::styled(
            center(status_text, status_w as usize),
            Style::default().fg(status_color),
        );

        let model_short = shorten_model(&t.last_model);
        let effort = effort_display(&t.last_effort);

        let ctx_used = t.last_ctx_used;
        let ctx_max_val = context_max(&t.last_model);
        let ctx_str = format!(
            "{}/{}",
            format_tokens(ctx_used),
            format_tokens(ctx_max_val)
        );

        let hit_rate = t.total_usage.hit_rate();
        let hit_color = if hit_rate >= 60.0 {
            GREEN
        } else if hit_rate >= 30.0 {
            YELLOW
        } else {
            RED
        };

        let duration_secs = (t.last_activity - t.first_activity).num_seconds().max(0);
        let hours = duration_secs / 3600;
        let mins = (duration_secs % 3600) / 60;

        let mut cells = Vec::new();

        cells.push(Cell::from(Span::styled(
            center(&pid_str, pid_w as usize),
            Style::default().fg(TEXT_HIGHLIGHT),
        )));

        if show_dir {
            cells.push(Cell::from(Span::styled(
                dir_display.clone(),
                Style::default().fg(TEXT_DIM),
            )));
        }

        // PROJECT cell: first 2 words of message (blue) + session id (gray)
        cells.push(Cell::from(Line::from(vec![
            Span::styled(project_display.0.clone(), Style::default().fg(BLUE)),
            Span::styled(format!(" {}", project_display.1), Style::default().fg(TEXT_IDLE)),
        ])));

        if show_status {
            cells.push(Cell::from(status_span));
        }
        if show_model {
            cells.push(Cell::from(Span::styled(
                center(&model_short, model_w as usize),
                Style::default().fg(TEXT_HIGHLIGHT),
            )));
        }
        if show_effort {
            cells.push(Cell::from(Span::styled(
                center(effort, effort_w as usize),
                Style::default().fg(effort_color(effort)),
            )));
        }
        if show_ctx {
            cells.push(Cell::from(Span::styled(
                center(&ctx_str, ctx_w as usize),
                Style::default().fg(BLUE),
            )));
        }
        if show_cache {
            cells.push(Cell::from(Span::styled(
                center(&format!("{:.0}%", hit_rate), cache_w as usize),
                Style::default().fg(hit_color),
            )));
        }
        if show_cost {
            cells.push(Cell::from(Span::styled(
                center(&format!("${:.2}", t.total_cost), cost_w as usize),
                Style::default().fg(ORANGE),
            )));
        }
        if show_duration {
            cells.push(Cell::from(center(
                &format!("{:02}:{:02}h", hours, mins),
                duration_w as usize,
            )));
        }

        let mut row_style = Style::default().bg(bg_color);
        if t.status == ThreadStatus::Idle || t.status == ThreadStatus::Error {
            row_style = row_style.fg(TEXT_IDLE);
        }

        Row::new(cells).style(row_style).height(1)
    });

    let visible_rows = area.height.saturating_sub(1) as usize; // 1 for header row
    let actual_offset = if selected >= scroll_offset + visible_rows {
        selected - visible_rows + 1
    } else if selected < scroll_offset {
        selected
    } else {
        scroll_offset
    };

    let table = Table::new(
        rows.skip(actual_offset)
            .take(visible_rows)
            .collect::<Vec<_>>(),
        widths,
    )
    .header(header)
    .block(Block::default().borders(Borders::NONE))
    .column_spacing(1);

    frame.render_widget(table, area);
}

fn effort_color(effort: &str) -> ratatui::style::Color {
    match effort {
        "Max" => ORANGE,
        "High" => YELLOW,
        "Low" => TEXT_DIM,
        _ => TEXT_HIGHLIGHT,
    }
}

/// Format project path as `root/../parent/last`.
/// root = `~` for home paths, `/first_component` for absolute paths.
/// Always guarantees the last two components are fully visible.
/// Never truncates from the right.
fn format_project_path(path: &str, max_w: usize) -> String {
    if path.is_empty() || max_w == 0 {
        return String::new();
    }

    let home = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();

    // Separate root from the rest of the components.
    // root = "~" for home paths, "/first" for absolute paths.
    let (root, comps): (String, Vec<&str>) =
        if !home.is_empty() && path.starts_with(&home) {
            let rest = &path[home.len()..];
            let c: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
            ("~".to_string(), c)
        } else {
            let all: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            if all.is_empty() {
                return "/".to_string();
            }
            // Root is /first_component; the rest are the middle+last components.
            (format!("/{}", all[0]), all[1..].to_vec())
        };

    // Build full path and return it if it fits.
    let full = if comps.is_empty() {
        root.clone()
    } else {
        format!("{}/{}", root, comps.join("/"))
    };
    if full.chars().count() <= max_w {
        return full;
    }

    // Path doesn't fit — abbreviate to: root/../second_last/last
    let n = comps.len();
    if n >= 2 {
        let abbr = format!("{}/../{}/{}", root, comps[n - 2], comps[n - 1]);
        if abbr.chars().count() <= max_w {
            return abbr;
        }
        // Still too wide — try root/../last
        let shorter = format!("{}/../{}", root, comps[n - 1]);
        if shorter.chars().count() <= max_w {
            return shorter;
        }
        // Last resort: just the final component, truncated
        return truncate_str(comps[n - 1], max_w);
    } else if n == 1 {
        // root/last
        let abbr = format!("{}/{}", root, comps[0]);
        if abbr.chars().count() <= max_w {
            return abbr;
        }
        return truncate_str(comps[0], max_w);
    }

    truncate_str(&root, max_w)
}

fn truncate_str(s: &str, max_w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_w {
        s.to_string()
    } else if max_w > 1 {
        format!("{}…", chars[..max_w - 1].iter().collect::<String>())
    } else {
        "…".to_string()
    }
}

/// Center a string within a fixed width using spaces.
fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let pad = width - len;
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}
