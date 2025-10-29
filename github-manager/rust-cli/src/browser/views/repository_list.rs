use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Row, Table},
    Frame,
};

use crate::browser::state::{SharedState, ViewType, Mode};
use crate::browser::views::View;
use crate::error::Result;
use crate::github::Repository;

pub struct RepositoryListView {
    state: SharedState,
    filter: String,
    sort_by: SortField,
    sort_ascending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortField {
    Name,
    Stars,
    Updated,
    Language,
}

impl RepositoryListView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            filter: String::new(),
            sort_by: SortField::Updated,
            sort_ascending: false,
        }
    }
    
    async fn get_filtered_repos(&self) -> Vec<Repository> {
        let state = self.state.read().await;
        let mut repos: Vec<Repository> = state.repositories.clone();
        
        // Apply filter
        if !self.filter.is_empty() {
            let filter_lower = self.filter.to_lowercase();
            repos.retain(|repo| {
                repo.name.to_lowercase().contains(&filter_lower) ||
                repo.description.as_ref()
                    .map(|d| d.to_lowercase().contains(&filter_lower))
                    .unwrap_or(false)
            });
        }
        
        // Apply sort
        match self.sort_by {
            SortField::Name => repos.sort_by(|a, b| {
                if self.sort_ascending {
                    a.name.cmp(&b.name)
                } else {
                    b.name.cmp(&a.name)
                }
            }),
            SortField::Stars => repos.sort_by(|a, b| {
                if self.sort_ascending {
                    a.stars.cmp(&b.stars)
                } else {
                    b.stars.cmp(&a.stars)
                }
            }),
            SortField::Updated => repos.sort_by(|a, b| {
                if self.sort_ascending {
                    a.updated_at.cmp(&b.updated_at)
                } else {
                    b.updated_at.cmp(&a.updated_at)
                }
            }),
            SortField::Language => repos.sort_by(|a, b| {
                let a_lang = a.language.as_deref().unwrap_or("");
                let b_lang = b.language.as_deref().unwrap_or("");
                if self.sort_ascending {
                    a_lang.cmp(b_lang)
                } else {
                    b_lang.cmp(a_lang)
                }
            }),
        }
        
        repos
    }
}

#[async_trait]
impl View for RepositoryListView {
    fn view_type(&self) -> ViewType {
        ViewType::RepositoryList
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Filter bar
                Constraint::Min(0),    // Repository list
            ])
            .split(area);
        
        // Draw filter bar
        let filter_block = Block::default()
            .borders(Borders::ALL)
            .title("Filter (/ to search)")
            .border_style(Style::default().fg(Color::DarkGray));
        
        let filter_text = if self.filter.is_empty() {
            Span::styled("Type to filter repositories...", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw(&self.filter)
        };
        
        let filter_widget = ratatui::widgets::Paragraph::new(Line::from(vec![
            Span::raw("🔍 "),
            filter_text,
        ]))
        .block(filter_block);
        
        frame.render_widget(filter_widget, chunks[0]);
        
        // Get filtered repositories
        let repos = self.get_filtered_repos().await;
        let state = self.state.read().await;
        let selected = state.list_state.selected;
        drop(state);
        
        // Create table
        let header_cells = ["Name", "Language", "Stars", "Updated", "Status"]
            .iter()
            .map(|h| {
                let style = if matches!(
                    (h, &self.sort_by),
                    (&"Name", SortField::Name) |
                    (&"Language", SortField::Language) |
                    (&"Stars", SortField::Stars) |
                    (&"Updated", SortField::Updated)
                ) {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::BOLD)
                };
                Cell::from(*h).style(style)
            });
        
        let header = Row::new(header_cells)
            .style(Style::default().bg(Color::DarkGray))
            .height(1);
        
        let rows = repos.iter().enumerate().map(|(i, repo)| {
            let status = if repo.private {
                Span::styled("🔒", Style::default().fg(Color::Red))
            } else {
                Span::styled("🌐", Style::default().fg(Color::Green))
            };
            
            let style = if i == selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            Row::new(vec![
                Cell::from(repo.name.clone()),
                Cell::from(repo.language.as_deref().unwrap_or("-")),
                Cell::from(format!("⭐ {}", repo.stars)),
                Cell::from(humantime::format_duration(
                    std::time::Duration::from_secs(
                        (chrono::Utc::now() - repo.updated_at).num_seconds() as u64
                    )
                ).to_string()),
                Cell::from(status),
            ])
            .style(style)
            .height(1)
        });
        
        let table = Table::new(rows)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Repositories ({}) ", repos.len()))
            )
            .widths(&[
                Constraint::Percentage(35),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
                Constraint::Percentage(15),
            ]);
        
        frame.render_widget(table, chunks[1]);
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        let mut state = self.state.write().await;
        
        match state.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('/') => {
                        state.mode = Mode::Insert;
                        self.filter.clear();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let repos = self.get_filtered_repos().await;
                        state.move_selection_down(repos.len());
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.move_selection_up();
                    }
                    KeyCode::Char('g') => {
                        state.list_state.selected = 0;
                    }
                    KeyCode::Char('G') => {
                        let repos = self.get_filtered_repos().await;
                        if !repos.is_empty() {
                            state.list_state.selected = repos.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        let repos = self.get_filtered_repos().await;
                        if let Some(repo) = repos.get(state.list_state.selected) {
                            state.selected_repository = Some(repo.clone());
                            state.navigate_to(ViewType::RepositoryDetail);
                        }
                    }
                    KeyCode::Char('s') => {
                        match (key.modifiers, self.sort_by) {
                            (KeyModifiers::NONE, _) => self.sort_by = SortField::Stars,
                            (KeyModifiers::SHIFT, SortField::Stars) => self.sort_ascending = !self.sort_ascending,
                            _ => {}
                        }
                    }
                    KeyCode::Char('n') => {
                        match (key.modifiers, self.sort_by) {
                            (KeyModifiers::NONE, _) => self.sort_by = SortField::Name,
                            (KeyModifiers::SHIFT, SortField::Name) => self.sort_ascending = !self.sort_ascending,
                            _ => {}
                        }
                    }
                    KeyCode::Char('u') => {
                        match (key.modifiers, self.sort_by) {
                            (KeyModifiers::NONE, _) => self.sort_by = SortField::Updated,
                            (KeyModifiers::SHIFT, SortField::Updated) => self.sort_ascending = !self.sort_ascending,
                            _ => {}
                        }
                    }
                    KeyCode::Char('l') => {
                        match (key.modifiers, self.sort_by) {
                            (KeyModifiers::NONE, _) => self.sort_by = SortField::Language,
                            (KeyModifiers::SHIFT, SortField::Language) => self.sort_ascending = !self.sort_ascending,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Mode::Insert => {
                match key.code {
                    KeyCode::Esc => {
                        state.mode = Mode::Normal;
                    }
                    KeyCode::Char(c) => {
                        self.filter.push(c);
                        state.list_state.selected = 0; // Reset selection on filter change
                    }
                    KeyCode::Backspace => {
                        self.filter.pop();
                        state.list_state.selected = 0;
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
    async fn test_repository_list_view() {
        let state = create_shared_state().await;
        let view = RepositoryListView::new(state.clone());
        
        assert_eq!(view.view_type(), ViewType::RepositoryList);
        assert_eq!(view.filter, "");
        assert_eq!(view.sort_by, SortField::Updated);
        assert!(!view.sort_ascending);
    }
}