use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Tabs},
};

pub struct Components;

impl Components {
    pub fn styled_block(title: &str, selected: bool) -> Block {
        let border_style = if selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style)
    }
    
    pub fn progress_bar(progress: f64, width: usize) -> String {
        let filled = (progress * width as f64) as usize;
        let empty = width.saturating_sub(filled);
        
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
    
    pub fn status_indicator(status: &str) -> Span {
        match status.to_lowercase().as_str() {
            "success" | "online" | "active" => {
                Span::styled("●", Style::default().fg(Color::Green))
            }
            "failed" | "error" | "offline" => {
                Span::styled("●", Style::default().fg(Color::Red))
            }
            "warning" | "pending" => {
                Span::styled("●", Style::default().fg(Color::Yellow))
            }
            _ => Span::styled("●", Style::default().fg(Color::Gray))
        }
    }
    
    pub fn truncate_middle(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else if max_width < 5 {
            text[..max_width].to_string()
        } else {
            let start_len = (max_width - 3) / 2;
            let end_len = max_width - 3 - start_len;
            format!("{}...{}", &text[..start_len], &text[text.len() - end_len..])
        }
    }
    
    pub fn format_duration(seconds: u64) -> String {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else if seconds < 86400 {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        } else {
            format!("{}d {}h", seconds / 86400, (seconds % 86400) / 3600)
        }
    }
}