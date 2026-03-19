use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Sparkline as TuiSparkline};
use ratatui::Frame;

use super::theme::{BG_BORDER, BLUE};

pub fn render_sparkline(frame: &mut Frame, area: Rect, data: &[u64]) {
    // If data holds the absolute total context, we might want to scale it or diff it.
    // The spec asks for total aggregated context tokens over 12 hours (refreshed 2s)
    let max = data.iter().max().copied().unwrap_or(1);
    
    let sparkline = TuiSparkline::default()
        .block(Block::default().title(" Token Usage Trend (agg context over time) ").borders(Borders::TOP).border_style(Style::default().fg(BG_BORDER)))
        .data(data)
        .max(max)
        .style(Style::default().fg(BLUE));

    frame.render_widget(sparkline, area);
}
