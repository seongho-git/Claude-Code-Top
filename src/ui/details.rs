use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::theme::{BG_BORDER, BLUE, GREEN, ORANGE, RED, TEXT_DIM, TEXT_HIGHLIGHT, TEXT_IDLE, YELLOW};
use crate::data::types::Session;

pub fn render_details(frame: &mut Frame, left_area: Rect, right_area: Rect, session: Option<&Session>) {
    if let Some(s) = session {
        // --- LEFT PANEL (Cost breakdown & Hit Rate) ---
        let mut left_lines = vec![];
        
        let display_name = s.project_path.rsplit('/').next().unwrap_or(&s.project_path);
        let status_type = if s.is_active { "local" } else { "offline" };
        left_lines.push(Line::from(vec![
            Span::styled(format!("▶ session {}  ~{}  [{}]", s.pid.unwrap_or(0), display_name, status_type), Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
        ]));
        left_lines.push(Line::from(""));
        
        // Cost Breakdown
        let usage = &s.total_usage;
        let c_in = usage.input_tokens;
        let c_out = usage.output_tokens;
        let c_cr = usage.cache_read_input_tokens;
        let c_cw = usage.cache_creation_input_tokens;
        
        left_lines.push(Line::from(format!("input tokens   {:>8}", c_in)));
        left_lines.push(Line::from(format!("output tokens  {:>8}", c_out)));
        left_lines.push(Line::from(""));
        
        let hit_rate = usage.hit_rate();
        left_lines.push(Line::from(vec![
            Span::styled(format!("cache reads    {:>8}   ({:.0}%)   ", c_cr, hit_rate), Style::default().fg(TEXT_IDLE)),
            Span::styled(format!("(saved ~${:.2})", s.saved_cost), Style::default().fg(GREEN)),
        ]));
        left_lines.push(Line::from(format!("cache writes   {:>8}", c_cw)));
        left_lines.push(Line::from(""));
        
        left_lines.push(Line::from(vec![
            Span::styled("total cost     ", Style::default().fg(TEXT_HIGHLIGHT)),
            Span::styled(format!("${:.2}   ", s.total_cost), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
            Span::styled(format!("(without cache: ~${:.2})", s.total_cost + s.saved_cost), Style::default().fg(TEXT_DIM)),
        ]));
        
        left_lines.push(Line::from(""));
        let bar_len: usize = 20;
        let filled_len = ((hit_rate / 100.0) * bar_len as f64).round() as usize;
        let bar_str = format!("[{}{}]", "█".repeat(filled_len), "░".repeat(bar_len.saturating_sub(filled_len)));
        let hit_color = if hit_rate >= 60.0 { GREEN } else if hit_rate >= 30.0 { YELLOW } else { RED };
        
        left_lines.push(Line::from(vec![
            Span::raw("hit rate bar   "),
            Span::styled(format!("{} {:.0}%", bar_str, hit_rate), Style::default().fg(hit_color)),
        ]));

        let left_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BG_BORDER))
            .title(Span::styled(" Session Details ", Style::default().fg(BLUE)));
        frame.render_widget(Paragraph::new(left_lines).block(left_block), left_area);

        // --- RIGHT PANEL (Tool Log & Cache info) ---
        let mut right_lines = vec![];
        right_lines.push(Line::from(Span::styled("tool activity log (placeholder)", Style::default().fg(TEXT_DIM))));
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled("14:08   read_file   src/routes/api.ts", Style::default().fg(TEXT_IDLE))));
        right_lines.push(Line::from(Span::styled("14:11   edit_file   src/routes/api.ts", Style::default().fg(TEXT_IDLE))));
        right_lines.push(Line::from(vec![Span::raw("14:15   bash        "), Span::styled("npm test", Style::default().fg(TEXT_IDLE))]));
        right_lines.push(Line::from(vec![Span::raw("14:19   bash        "), Span::styled("✓ 14 passed", Style::default().fg(GREEN))]));
        
        // Cache Box
        let box_y = right_area.height.saturating_sub(7);
        if right_area.height >= 10 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(box_y), Constraint::Min(6)])
                .split(right_area);
            
            let right_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BG_BORDER))
                .title(Span::styled(" Tool Log ", Style::default().fg(BLUE)));
            frame.render_widget(Paragraph::new(right_lines).block(right_block), chunks[0]);

            let cache_box = vec![
                Line::from(Span::styled(" 시스템 프롬프트 + 파일 컨텍스트가 반복될 때 캐시 로드.", Style::default().fg(TEXT_IDLE))),
                Line::from(Span::styled(" read  0.10× 단가  →  90% 절감", Style::default().fg(GREEN))),
                Line::from(Span::styled(" write 1.25× 단가  →  첫 저장 비용", Style::default().fg(ORANGE))),
                Line::from(""),
                Line::from(Span::styled(format!(" 이 세션 절감액  ${:.2}", s.saved_cost), Style::default().fg(TEXT_HIGHLIGHT))),
            ];
            let cache_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BG_BORDER))
                .title(Span::styled(format!(" cache hit rate: {:.0}% ", hit_rate), Style::default().fg(BLUE)));
            frame.render_widget(Paragraph::new(cache_box).block(cache_block), chunks[1]);
        } else {
            let right_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BG_BORDER))
                .title(Span::styled(" Tool Log ", Style::default().fg(BLUE)));
            frame.render_widget(Paragraph::new(right_lines).block(right_block), right_area);
        }

    } else {
        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BG_BORDER));
        frame.render_widget(Paragraph::new("No session selected").block(block.clone()), left_area);
        frame.render_widget(Paragraph::new("No session selected").block(block), right_area);
    }
}
