use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{ThemeSet, Style as SyntectStyle},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use std::path::Path;

use crate::browser::state::{SharedState, ViewType, Mode};
use crate::browser::views::View;
use crate::error::Result;

pub struct CodeView {
    state: SharedState,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    scroll_offset: usize,
    horizontal_offset: usize,
    search_term: Option<String>,
    search_results: Vec<usize>,
    current_search_index: usize,
}

impl CodeView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            scroll_offset: 0,
            horizontal_offset: 0,
            search_term: None,
            search_results: Vec::new(),
            current_search_index: 0,
        }
    }
    
    fn get_syntax_for_file(&self, path: &Path) -> &syntect::parsing::SyntaxReference {
        self.syntax_set
            .find_syntax_for_file(path)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }
    
    fn highlight_line(&self, line: &str, highlighter: &mut HighlightLines) -> Vec<Span<'static>> {
        let ranges = highlighter.highlight_line(line, &self.syntax_set).unwrap();
        
        ranges
            .into_iter()
            .map(|(style, text)| {
                let fg = style.foreground;
                let color = Color::Rgb(fg.r, fg.g, fg.b);
                
                let mut ratatui_style = Style::default().fg(color);
                
                if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                }
                
                Span::styled(text.to_string(), ratatui_style)
            })
            .collect()
    }
    
    fn perform_search(&mut self, content: &str) {
        self.search_results.clear();
        
        if let Some(term) = &self.search_term {
            if !term.is_empty() {
                let term_lower = term.to_lowercase();
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&term_lower) {
                        self.search_results.push(i);
                    }
                }
            }
        }
        
        self.current_search_index = 0;
    }
    
    fn jump_to_next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            if let Some(&line_num) = self.search_results.get(self.current_search_index) {
                self.scroll_offset = line_num.saturating_sub(5); // Center the result
            }
        }
    }
    
    fn jump_to_previous_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = if self.current_search_index == 0 {
                self.search_results.len() - 1
            } else {
                self.current_search_index - 1
            };
            if let Some(&line_num) = self.search_results.get(self.current_search_index) {
                self.scroll_offset = line_num.saturating_sub(5);
            }
        }
    }
}

#[async_trait]
impl View for CodeView {
    fn view_type(&self) -> ViewType {
        ViewType::Code
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Code content
                Constraint::Length(3), // Status bar
            ])
            .split(area);
        
        let state = self.state.read().await;
        
        // Draw header
        let header_text = if let Some(path) = &state.current_file_path {
            format!("{}", path.display())
        } else {
            "No file selected".to_string()
        };
        
        let header = Paragraph::new(Line::from(vec![
            Span::raw("📄 "),
            Span::styled(header_text, Style::default().add_modifier(Modifier::BOLD)),
            if let Some(term) = &self.search_term {
                Span::styled(
                    format!(" | Search: {}", term),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            },
        ]))
        .block(Block::default().borders(Borders::ALL).title("Code View"));
        
        frame.render_widget(header, chunks[0]);
        
        // Draw code content
        if let Some(content) = &state.current_file_content {
            let syntax = if let Some(path) = &state.current_file_path {
                self.get_syntax_for_file(path)
            } else {
                self.syntax_set.find_syntax_plain_text()
            };
            
            let theme = &self.theme_set.themes["base16-ocean.dark"];
            let mut highlighter = HighlightLines::new(syntax, theme);
            
            let lines: Vec<&str> = content.lines().collect();
            let visible_lines = lines
                .iter()
                .skip(self.scroll_offset)
                .take(chunks[1].height as usize - 2); // Account for borders
            
            let mut highlighted_lines: Vec<Line> = Vec::new();
            let line_number_width = (self.scroll_offset + chunks[1].height as usize)
                .to_string()
                .len()
                .max(4);
            
            for (i, line) in visible_lines.enumerate() {
                let line_num = self.scroll_offset + i;
                let is_search_result = self.search_results.contains(&line_num);
                let is_current_search = self.search_results.get(self.current_search_index) == Some(&line_num);
                
                // Line number
                let line_number_style = if is_current_search {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else if is_search_result {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                
                let mut spans = vec![
                    Span::styled(
                        format!("{:>width$} ", line_num + 1, width = line_number_width),
                        line_number_style,
                    ),
                    Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                ];
                
                // Highlighted code
                let code_spans = self.highlight_line(line, &mut highlighter);
                
                // Apply horizontal scrolling
                let mut char_count = 0;
                for span in code_spans {
                    if char_count >= self.horizontal_offset {
                        let visible_text: String = span
                            .content
                            .chars()
                            .skip(self.horizontal_offset.saturating_sub(char_count))
                            .collect();
                        
                        if is_search_result && self.search_term.as_ref().map_or(false, |term| {
                            visible_text.to_lowercase().contains(&term.to_lowercase())
                        }) {
                            // Highlight search term
                            spans.push(Span::styled(visible_text, span.style.bg(Color::Yellow)));
                        } else {
                            spans.push(Span::styled(visible_text, span.style));
                        }
                    }
                    char_count += span.content.chars().count();
                }
                
                highlighted_lines.push(Line::from(spans));
            }
            
            let code_widget = Paragraph::new(highlighted_lines)
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            
            frame.render_widget(code_widget, chunks[1]);
        } else {
            let no_content = Paragraph::new("No file content to display")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(no_content, chunks[1]);
        }
        
        // Draw status bar
        let total_lines = state.current_file_content.as_ref()
            .map(|c| c.lines().count())
            .unwrap_or(0);
        
        let status_text = vec![
            Span::raw(format!("Line {}/{} ", self.scroll_offset + 1, total_lines)),
            Span::raw("| "),
            Span::raw(format!("Col {} ", self.horizontal_offset + 1)),
            if !self.search_results.is_empty() {
                Span::styled(
                    format!("| Match {}/{} ", self.current_search_index + 1, self.search_results.len()),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            },
            Span::raw("| "),
            Span::styled(
                "j/k: Scroll | h/l: H-Scroll | /: Search | n/N: Next/Prev | q: Back",
                Style::default().fg(Color::DarkGray),
            ),
        ];
        
        let status_bar = Paragraph::new(Line::from(status_text))
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(status_bar, chunks[2]);
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        let mut state = self.state.write().await;
        
        match state.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        let total_lines = state.current_file_content.as_ref()
                            .map(|c| c.lines().count())
                            .unwrap_or(0);
                        if self.scroll_offset < total_lines.saturating_sub(1) {
                            self.scroll_offset += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if self.horizontal_offset > 0 {
                            self.horizontal_offset -= 1;
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        self.horizontal_offset += 1;
                    }
                    KeyCode::Char('g') => {
                        self.scroll_offset = 0;
                    }
                    KeyCode::Char('G') => {
                        let total_lines = state.current_file_content.as_ref()
                            .map(|c| c.lines().count())
                            .unwrap_or(0);
                        self.scroll_offset = total_lines.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.scroll_offset += 20;
                        let total_lines = state.current_file_content.as_ref()
                            .map(|c| c.lines().count())
                            .unwrap_or(0);
                        if self.scroll_offset >= total_lines {
                            self.scroll_offset = total_lines.saturating_sub(1);
                        }
                    }
                    KeyCode::PageUp => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(20);
                    }
                    KeyCode::Char('/') => {
                        state.mode = Mode::Command;
                        state.command_buffer = "/".to_string();
                    }
                    KeyCode::Char('n') => {
                        self.jump_to_next_search_result();
                    }
                    KeyCode::Char('N') => {
                        self.jump_to_previous_search_result();
                    }
                    KeyCode::Char('0') => {
                        self.horizontal_offset = 0;
                    }
                    KeyCode::Char('$') => {
                        // Jump to end of longest line
                        if let Some(content) = &state.current_file_content {
                            let max_len = content.lines()
                                .map(|l| l.chars().count())
                                .max()
                                .unwrap_or(0);
                            self.horizontal_offset = max_len.saturating_sub(80); // Assuming ~80 char width
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        state.navigate_back();
                    }
                    _ => {}
                }
            }
            Mode::Command => {
                match key.code {
                    KeyCode::Esc => {
                        state.mode = Mode::Normal;
                        state.command_buffer.clear();
                    }
                    KeyCode::Enter => {
                        if state.command_buffer.starts_with('/') {
                            self.search_term = Some(state.command_buffer[1..].to_string());
                            if let Some(content) = &state.current_file_content {
                                self.perform_search(content);
                            }
                        }
                        state.mode = Mode::Normal;
                        state.command_buffer.clear();
                    }
                    KeyCode::Char(c) => {
                        state.command_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        if state.command_buffer.len() > 1 {
                            state.command_buffer.pop();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        Ok(false)
    }
}