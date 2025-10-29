pub mod components;
pub mod handlers;
pub mod state;
pub mod views;

use std::sync::Arc;
use tokio::sync::Mutex;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{config::Config, error::Result};
use self::{
    state::{BrowserState, ViewMode},
    views::{View, ViewType},
};

pub struct Browser {
    state: Arc<Mutex<BrowserState>>,
    config: Config,
    views: Vec<Box<dyn View>>,
    current_view: usize,
}

impl Browser {
    pub async fn new(config: Config) -> Result<Self> {
        let state = Arc::new(Mutex::new(BrowserState::new()));
        
        // Initialize views
        let views: Vec<Box<dyn View>> = vec![
            Box::new(views::RepositoryListView::new(state.clone())),
            Box::new(views::RepositoryDetailView::new(state.clone())),
            Box::new(views::FileExplorerView::new(state.clone())),
            Box::new(views::CodeView::new(state.clone())),
            Box::new(views::SearchView::new(state.clone())),
            Box::new(views::AnalyticsView::new(state.clone())),
            Box::new(views::WorkflowView::new(state.clone())),
            Box::new(views::SecurityView::new(state.clone())),
            Box::new(views::HelpView::new()),
        ];
        
        Ok(Self {
            state,
            config,
            views,
            current_view: 0,
        })
    }
    
    pub async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>) -> Result<()> {
        let size = frame.size();
        
        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // Header
                Constraint::Min(0),      // Content
                Constraint::Length(3),   // Status bar
            ])
            .split(size);
        
        // Draw header
        self.draw_header(frame, chunks[0]).await?;
        
        // Draw current view
        let state = self.state.lock().await;
        let view_type = state.current_view;
        drop(state); // Release lock
        
        // Find and draw the appropriate view
        for view in &mut self.views {
            if view.view_type() == view_type {
                view.draw(frame, chunks[1]).await?;
                break;
            }
        }
        
        // Draw status bar
        self.draw_status_bar(frame, chunks[2]).await?;
        
        Ok(())
    }
    
    pub async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        // Global key handlers
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => return Ok(true), // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.switch_view(ViewType::Help).await?;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
                self.refresh().await?;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.switch_view(ViewType::Search).await?;
            }
            (KeyModifiers::ALT, KeyCode::Char(n)) if n.is_numeric() => {
                let index = n.to_digit(10).unwrap_or(0) as usize;
                if index > 0 && index <= self.views.len() {
                    self.current_view = index - 1;
                    let view_type = self.views[self.current_view].view_type();
                    self.switch_view(view_type).await?;
                }
            }
            _ => {
                // Pass to current view
                let mut state = self.state.lock().await;
                let view_type = state.current_view;
                drop(state);
                
                for view in &mut self.views {
                    if view.view_type() == view_type {
                        return view.handle_input(key).await;
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    async fn switch_view(&mut self, view_type: ViewType) -> Result<()> {
        let mut state = self.state.lock().await;
        state.current_view = view_type;
        
        // Update view index
        for (i, view) in self.views.iter().enumerate() {
            if view.view_type() == view_type {
                self.current_view = i;
                break;
            }
        }
        
        Ok(())
    }
    
    async fn refresh(&mut self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.refresh().await?;
        Ok(())
    }
    
    async fn draw_header<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph},
        };
        
        let state = self.state.lock().await;
        let title = format!(" LlamaSearchAI Repository Browser - {} ", state.current_view);
        
        let header = Paragraph::new(Line::from(vec![
            Span::styled("⚡", Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        
        frame.render_widget(header, area);
        Ok(())
    }
    
    async fn draw_status_bar<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        use ratatui::{
            style::{Color, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph},
        };
        
        let state = self.state.lock().await;
        
        // Build status text
        let mut spans = vec![];
        
        // Mode indicator
        spans.push(Span::styled(
            format!(" {} ", state.mode),
            Style::default().bg(Color::Blue).fg(Color::White),
        ));
        spans.push(Span::raw(" "));
        
        // Repository count
        spans.push(Span::raw(format!("Repos: {} ", state.repositories.len())));
        
        // Loading indicator
        if state.loading {
            spans.push(Span::styled(
                "⟳ Loading... ",
                Style::default().fg(Color::Yellow),
            ));
        }
        
        // Help text
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            "^Q: Quit | ^H: Help | ^S: Search | Alt+[1-9]: Switch View",
            Style::default().fg(Color::DarkGray),
        ));
        
        let status = Paragraph::new(Line::from(spans))
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(status, area);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_browser_creation() {
        let config = Config::default();
        let browser = Browser::new(config).await.unwrap();
        assert_eq!(browser.current_view, 0);
        assert_eq!(browser.views.len(), 9);
    }
}