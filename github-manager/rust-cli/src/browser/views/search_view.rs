use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::browser::state::{SharedState, ViewType, Mode, SearchResult};
use crate::browser::views::View;
use crate::error::Result;

pub struct SearchView {
    state: SharedState,
    search_input: String,
    selected_result: usize,
    search_filters: SearchFilters,
}

#[derive(Debug, Clone)]
struct SearchFilters {
    case_sensitive: bool,
    regex: bool,
    whole_word: bool,
    file_types: Vec<String>,
    repositories: Vec<String>,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            regex: false,
            whole_word: false,
            file_types: Vec::new(),
            repositories: Vec::new(),
        }
    }
}

impl SearchView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            search_input: String::new(),
            selected_result: 0,
            search_filters: SearchFilters::default(),
        }
    }
}

#[async_trait]
impl View for SearchView {
    fn view_type(&self) -> ViewType {
        ViewType::Search
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Length(3), // Filters
                Constraint::Min(0),    // Results
                Constraint::Length(3), // Status
            ])
            .split(area);
        
        let state = self.state.read().await;
        
        // Draw search input
        let search_input = Paragraph::new(Line::from(vec![
            Span::raw("🔍 "),
            if self.search_input.is_empty() {
                Span::styled("Enter search query...", Style::default().fg(Color::DarkGray))
            } else {
                Span::raw(&self.search_input)
            },
            if matches!(state.mode, Mode::Insert) {
                Span::styled("█", Style::default().add_modifier(Modifier::SLOW_BLINK))
            } else {
                Span::raw("")
            },
        ]))
        .block(Block::default().borders(Borders::ALL).title("Search"));
        
        frame.render_widget(search_input, chunks[0]);
        
        // Draw filters
        let mut filter_spans = vec![Span::raw("Filters: ")];
        
        if self.search_filters.case_sensitive {
            filter_spans.push(Span::styled("[Case] ", Style::default().fg(Color::Yellow)));
        }
        if self.search_filters.regex {
            filter_spans.push(Span::styled("[Regex] ", Style::default().fg(Color::Yellow)));
        }
        if self.search_filters.whole_word {
            filter_spans.push(Span::styled("[Word] ", Style::default().fg(Color::Yellow)));
        }
        if !self.search_filters.file_types.is_empty() {
            filter_spans.push(Span::styled(
                format!("[Types: {}] ", self.search_filters.file_types.join(",")),
                Style::default().fg(Color::Yellow),
            ));
        }
        
        filter_spans.push(Span::styled(
            " | c: Case | r: Regex | w: Word | t: Types",
            Style::default().fg(Color::DarkGray),
        ));
        
        let filters = Paragraph::new(Line::from(filter_spans))
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(filters, chunks[1]);
        
        // Draw search results
        if state.search_in_progress {
            let loading = Paragraph::new("⟳ Searching...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Results"));
            frame.render_widget(loading, chunks[2]);
        } else if !state.search_results.is_empty() {
            let results: Vec<ListItem> = state
                .search_results
                .iter()
                .enumerate()
                .map(|(i, result)| {
                    let style = if i == self.selected_result {
                        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    
                    let repo_style = Style::default().fg(Color::Blue);
                    let path_style = Style::default().fg(Color::Green);
                    let line_style = Style::default().fg(Color::Yellow);
                    
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(&result.repository, repo_style),
                            Span::raw(":"),
                            Span::styled(&result.file_path, path_style),
                            Span::raw(":"),
                            Span::styled(result.line_number.to_string(), line_style),
                        ]),
                        Line::from(vec![
                            Span::raw("  "),
                            Span::raw(&result.content),
                        ]),
                    ])
                    .style(style)
                })
                .collect();
            
            let results_list = List::new(results)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Results ({})", state.search_results.len())));
            
            frame.render_widget(results_list, chunks[2]);
        } else if !self.search_input.is_empty() {
            let no_results = Paragraph::new("No results found")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Results"));
            frame.render_widget(no_results, chunks[2]);
        } else {
            let hint = Paragraph::new(vec![
                Line::from(""),
                Line::from("Enter a search query to find code across all repositories"),
                Line::from(""),
                Line::from("Tips:"),
                Line::from("  • Use regular expressions with the 'r' filter"),
                Line::from("  • Filter by file type with the 't' filter"),
                Line::from("  • Press '/' to start searching"),
            ])
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Results"));
            frame.render_widget(hint, chunks[2]);
        }
        
        // Draw status bar
        let status_text = if !state.search_results.is_empty() {
            format!(
                "Result {}/{} | Enter: Open | /: New Search | Esc: Cancel",
                self.selected_result + 1,
                state.search_results.len()
            )
        } else {
            "/: Search | Esc: Back".to_string()
        };
        
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(status, chunks[3]);
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        let mut state = self.state.write().await;
        
        match state.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('/') => {
                        state.mode = Mode::Insert;
                        self.search_input.clear();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.selected_result < state.search_results.len().saturating_sub(1) {
                            self.selected_result += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.selected_result > 0 {
                            self.selected_result -= 1;
                        }
                    }
                    KeyCode::Char('g') => {
                        self.selected_result = 0;
                    }
                    KeyCode::Char('G') => {
                        if !state.search_results.is_empty() {
                            self.selected_result = state.search_results.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(result) = state.search_results.get(self.selected_result) {
                            // Open the file at the specific line
                            state.current_file_path = Some(result.file_path.clone().into());
                            // In a real implementation, we'd also set the scroll offset
                            state.navigate_to(ViewType::Code);
                        }
                    }
                    KeyCode::Char('c') => {
                        self.search_filters.case_sensitive = !self.search_filters.case_sensitive;
                    }
                    KeyCode::Char('r') => {
                        self.search_filters.regex = !self.search_filters.regex;
                    }
                    KeyCode::Char('w') => {
                        self.search_filters.whole_word = !self.search_filters.whole_word;
                    }
                    KeyCode::Char('t') => {
                        // In a real implementation, this would open a file type selector
                        state.set_status(
                            "File type filtering coming soon".to_string(),
                            crate::browser::state::StatusMessageType::Info,
                        );
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        state.navigate_back();
                    }
                    _ => {}
                }
            }
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => {
                        state.mode = Mode::Normal;
                    }
                    KeyCode::Enter => {
                        state.mode = Mode::Normal;
                        if !self.search_input.is_empty() {
                            // Trigger search
                            state.search_query = self.search_input.clone();
                            state.search_in_progress = true;
                            self.selected_result = 0;
                            
                            // In a real implementation, this would trigger an async search
                            // For now, we'll add some mock results
                            state.search_results = vec![
                                SearchResult {
                                    file_path: "src/main.rs".to_string(),
                                    line_number: 42,
                                    content: "    let result = search_function(&query);".to_string(),
                                    repository: "llamaagent".to_string(),
                                },
                                SearchResult {
                                    file_path: "src/lib.rs".to_string(),
                                    line_number: 123,
                                    content: "pub fn search_function(query: &str) -> Vec<Result> {".to_string(),
                                    repository: "llamaagent".to_string(),
                                },
                                SearchResult {
                                    file_path: "tests/search_test.rs".to_string(),
                                    line_number: 15,
                                    content: "    assert_eq!(search_function(\"test\"), expected);".to_string(),
                                    repository: "llamaagent".to_string(),
                                },
                            ];
                            state.search_in_progress = false;
                        }
                    }
                    KeyCode::Char(c) => {
                        self.search_input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.search_input.pop();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::state::create_shared_state;
    
    #[tokio::test]
    async fn test_search_view() {
        let state = create_shared_state().await;
        let view = SearchView::new(state.clone());
        
        assert_eq!(view.view_type(), ViewType::Search);
        assert_eq!(view.search_input, "");
        assert_eq!(view.selected_result, 0);
        assert!(!view.search_filters.case_sensitive);
        assert!(!view.search_filters.regex);
    }
}