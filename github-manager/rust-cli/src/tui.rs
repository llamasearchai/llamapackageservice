use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::error::Result;

pub struct Tui;

impl Tui {
    pub fn draw_popup<B: Backend>(
        frame: &mut Frame<B>,
        title: &str,
        content: Vec<Line<'static>>,
        width_percent: u16,
        height_percent: u16,
    ) {
        let size = frame.size();
        let popup_width = size.width * width_percent / 100;
        let popup_height = size.height * height_percent / 100;
        
        let popup = Rect {
            x: (size.width - popup_width) / 2,
            y: (size.height - popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };
        
        frame.render_widget(Clear, popup);
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        
        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true });
        
        frame.render_widget(paragraph, popup);
    }
    
    pub fn draw_error_popup<B: Backend>(frame: &mut Frame<B>, error: &str) {
        let content = vec![
            Line::from(""),
            Line::from(Span::styled("Error:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from(Span::styled("Press any key to continue", Style::default().fg(Color::DarkGray))),
        ];
        
        Self::draw_popup(frame, " Error ", content, 60, 30);
    }
    
    pub fn draw_loading<B: Backend>(frame: &mut Frame<B>, message: &str) {
        let content = vec![
            Line::from(""),
            Line::from("⟳ Loading..."),
            Line::from(""),
            Line::from(message),
        ];
        
        Self::draw_popup(frame, " Loading ", content, 40, 20);
    }
    
    pub fn draw_confirm_dialog<B: Backend>(
        frame: &mut Frame<B>,
        title: &str,
        message: &str,
        confirm_key: &str,
        cancel_key: &str,
    ) {
        let content = vec![
            Line::from(""),
            Line::from(message),
            Line::from(""),
            Line::from(vec![
                Span::styled(confirm_key, Style::default().fg(Color::Green)),
                Span::raw(" to confirm, "),
                Span::styled(cancel_key, Style::default().fg(Color::Red)),
                Span::raw(" to cancel"),
            ]),
        ];
        
        Self::draw_popup(frame, title, content, 50, 25);
    }
    
    pub fn draw_input_dialog<B: Backend>(
        frame: &mut Frame<B>,
        title: &str,
        prompt: &str,
        value: &str,
        cursor_visible: bool,
    ) {
        let mut content = vec![
            Line::from(""),
            Line::from(prompt),
            Line::from(""),
        ];
        
        let input_line = if cursor_visible {
            Line::from(vec![
                Span::raw("> "),
                Span::raw(value),
                Span::styled("█", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ])
        } else {
            Line::from(vec![
                Span::raw("> "),
                Span::raw(value),
            ])
        };
        
        content.push(input_line);
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Enter to confirm, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )));
        
        Self::draw_popup(frame, title, content, 60, 30);
    }
    
    pub fn draw_progress<B: Backend>(
        frame: &mut Frame<B>,
        title: &str,
        current: usize,
        total: usize,
        message: &str,
    ) {
        let percentage = if total > 0 {
            (current as f64 / total as f64 * 100.0) as u16
        } else {
            0
        };
        
        let progress_bar = "█".repeat((percentage / 5) as usize);
        let remaining = "░".repeat(20 - (percentage / 5) as usize);
        
        let content = vec![
            Line::from(""),
            Line::from(message),
            Line::from(""),
            Line::from(vec![
                Span::styled(progress_bar, Style::default().fg(Color::Green)),
                Span::styled(remaining, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(format!("{}/{} ({}%)", current, total, percentage)),
        ];
        
        Self::draw_popup(frame, title, content, 50, 25);
    }
    
    pub fn draw_keybinds<B: Backend>(frame: &mut Frame<B>, keybinds: Vec<(&str, &str)>) {
        let content: Vec<Line> = keybinds
            .into_iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(format!("{:<10}", key), Style::default().fg(Color::Yellow)),
                    Span::raw(desc),
                ])
            })
            .collect();
        
        Self::draw_popup(frame, " Keyboard Shortcuts ", content, 60, 50);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_centered_rect() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        
        let centered = centered_rect(50, 50, area);
        
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }
}