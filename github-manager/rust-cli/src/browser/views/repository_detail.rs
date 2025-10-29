use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::browser::state::{SharedState, ViewType, Mode};
use crate::browser::views::View;
use crate::error::Result;

pub struct RepositoryDetailView {
    state: SharedState,
    active_tab: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DetailTab {
    Overview,
    Files,
    Issues,
    PullRequests,
    Actions,
    Settings,
}

impl RepositoryDetailView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            active_tab: 0,
        }
    }
    
    fn get_tabs() -> Vec<&'static str> {
        vec!["Overview", "Files", "Issues", "Pull Requests", "Actions", "Settings"]
    }
}

#[async_trait]
impl View for RepositoryDetailView {
    fn view_type(&self) -> ViewType {
        ViewType::RepositoryDetail
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let state = self.state.read().await;
        
        if let Some(repo) = &state.selected_repository {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Repository header
                    Constraint::Length(3), // Tabs
                    Constraint::Min(0),    // Content
                ])
                .split(area);
            
            // Draw repository header
            let header = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(&repo.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" - "),
                    Span::raw(repo.description.as_deref().unwrap_or("No description")),
                ]),
                Line::from(vec![
                    Span::raw("⭐ "),
                    Span::styled(repo.stars.to_string(), Style::default().fg(Color::Yellow)),
                    Span::raw(" | 🍴 "),
                    Span::raw(repo.forks.to_string()),
                    Span::raw(" | 👁 "),
                    Span::raw(repo.watchers.to_string()),
                    Span::raw(" | "),
                    Span::styled(
                        repo.language.as_deref().unwrap_or("Unknown"),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
            ])
            .block(Block::default().borders(Borders::ALL));
            
            frame.render_widget(header, chunks[0]);
            
            // Draw tabs
            let tabs = Tabs::new(Self::get_tabs())
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .select(self.active_tab);
            
            frame.render_widget(tabs, chunks[1]);
            
            // Draw content based on active tab
            match self.active_tab {
                0 => self.draw_overview(frame, chunks[2], repo).await?,
                1 => self.draw_files(frame, chunks[2]).await?,
                2 => self.draw_issues(frame, chunks[2]).await?,
                3 => self.draw_pull_requests(frame, chunks[2]).await?,
                4 => self.draw_actions(frame, chunks[2]).await?,
                5 => self.draw_settings(frame, chunks[2], repo).await?,
                _ => {}
            }
        } else {
            let msg = Paragraph::new("No repository selected")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(msg, area);
        }
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        let mut state = self.state.write().await;
        
        match state.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Tab => {
                        self.active_tab = (self.active_tab + 1) % Self::get_tabs().len();
                    }
                    KeyCode::BackTab => {
                        if self.active_tab > 0 {
                            self.active_tab -= 1;
                        } else {
                            self.active_tab = Self::get_tabs().len() - 1;
                        }
                    }
                    KeyCode::Char('1') => self.active_tab = 0,
                    KeyCode::Char('2') => self.active_tab = 1,
                    KeyCode::Char('3') => self.active_tab = 2,
                    KeyCode::Char('4') => self.active_tab = 3,
                    KeyCode::Char('5') => self.active_tab = 4,
                    KeyCode::Char('6') => self.active_tab = 5,
                    KeyCode::Enter => {
                        if self.active_tab == 1 {
                            // Navigate to file explorer
                            state.navigate_to(ViewType::FileExplorer);
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        state.navigate_back();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        Ok(false)
    }
}

impl RepositoryDetailView {
    async fn draw_overview<B: Backend>(
        &self,
        frame: &mut Frame<B>,
        area: Rect,
        repo: &crate::github::Repository,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Stats
                Constraint::Length(8),  // Activity
                Constraint::Min(0),     // Recent commits
            ])
            .split(area);
        
        // Statistics
        let stats_block = Block::default()
            .title("Statistics")
            .borders(Borders::ALL);
        
        let inner = stats_block.inner(chunks[0]);
        frame.render_widget(stats_block, chunks[0]);
        
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(inner);
        
        // Draw mini stats
        for (i, (label, value, color)) in [
            ("Open Issues", repo.open_issues_count.to_string(), Color::Red),
            ("Open PRs", "0".to_string(), Color::Blue), // Would come from API
            ("Size", format!("{} KB", repo.size), Color::Green),
            ("License", repo.license.as_ref().map(|l| l.name.as_str()).unwrap_or("None").to_string(), Color::Yellow),
        ].iter().enumerate() {
            let stat = Paragraph::new(vec![
                Line::from(Span::styled(label, Style::default().add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(value, Style::default().fg(*color))),
            ])
            .alignment(Alignment::Center);
            frame.render_widget(stat, stat_chunks[i]);
        }
        
        // Activity graph (placeholder)
        let activity_gauge = Gauge::default()
            .block(Block::default().title("Activity (Last 30 days)").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(75)
            .label("75% active");
        frame.render_widget(activity_gauge, chunks[1]);
        
        // Recent activity
        let recent_items: Vec<ListItem> = vec![
            ListItem::new("🔀 Merged PR #123: Update documentation"),
            ListItem::new("💬 New issue #456: Bug in parser"),
            ListItem::new("⚡ Push to main: Fix critical bug"),
            ListItem::new("🏷️ New release: v2.1.0"),
            ListItem::new("🔧 Updated CI/CD pipeline"),
        ];
        
        let recent_list = List::new(recent_items)
            .block(Block::default().title("Recent Activity").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        
        frame.render_widget(recent_list, chunks[2]);
        
        Ok(())
    }
    
    async fn draw_files<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to explore files",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from("This will open the file explorer for this repository"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().title("Files").borders(Borders::ALL));
        
        frame.render_widget(hint, area);
        Ok(())
    }
    
    async fn draw_issues<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let state = self.state.read().await;
        
        // Placeholder issue list
        let issues: Vec<ListItem> = vec![
            ListItem::new(Line::from(vec![
                Span::styled("🔴 ", Style::default()),
                Span::styled("#123", Style::default().fg(Color::Blue)),
                Span::raw(" Bug: Memory leak in parser"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🟡 ", Style::default()),
                Span::styled("#124", Style::default().fg(Color::Blue)),
                Span::raw(" Feature: Add support for async"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🟢 ", Style::default()),
                Span::styled("#125", Style::default().fg(Color::Blue)),
                Span::raw(" Enhancement: Improve documentation"),
            ])),
        ];
        
        let list = List::new(issues)
            .block(Block::default().title("Issues").borders(Borders::ALL))
            .style(Style::default());
        
        frame.render_widget(list, area);
        Ok(())
    }
    
    async fn draw_pull_requests<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let prs: Vec<ListItem> = vec![
            ListItem::new(Line::from(vec![
                Span::styled("✅ ", Style::default()),
                Span::styled("#45", Style::default().fg(Color::Blue)),
                Span::raw(" Fix: Correct typo in README"),
                Span::styled(" +10 -2", Style::default().fg(Color::Green)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🔄 ", Style::default()),
                Span::styled("#46", Style::default().fg(Color::Blue)),
                Span::raw(" WIP: Refactor authentication"),
                Span::styled(" +156 -89", Style::default().fg(Color::Yellow)),
            ])),
        ];
        
        let list = List::new(prs)
            .block(Block::default().title("Pull Requests").borders(Borders::ALL))
            .style(Style::default());
        
        frame.render_widget(list, area);
        Ok(())
    }
    
    async fn draw_actions<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let workflows: Vec<ListItem> = vec![
            ListItem::new(Line::from(vec![
                Span::styled("✅ ", Style::default().fg(Color::Green)),
                Span::raw("CI/CD Pipeline"),
                Span::raw(" - "),
                Span::styled("Success", Style::default().fg(Color::Green)),
                Span::raw(" (2m ago)"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🔄 ", Style::default().fg(Color::Yellow)),
                Span::raw("Security Scan"),
                Span::raw(" - "),
                Span::styled("Running", Style::default().fg(Color::Yellow)),
                Span::raw(" (30s)"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::raw("Deploy to Production"),
                Span::raw(" - "),
                Span::styled("Failed", Style::default().fg(Color::Red)),
                Span::raw(" (1h ago)"),
            ])),
        ];
        
        let list = List::new(workflows)
            .block(Block::default().title("GitHub Actions").borders(Borders::ALL))
            .style(Style::default());
        
        frame.render_widget(list, area);
        Ok(())
    }
    
    async fn draw_settings<B: Backend>(
        &self,
        frame: &mut Frame<B>,
        area: Rect,
        repo: &crate::github::Repository,
    ) -> Result<()> {
        let settings_text = vec![
            Line::from(vec![
                Span::styled("Visibility: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if repo.private { "Private 🔒" } else { "Public 🌐" },
                    Style::default().fg(if repo.private { Color::Red } else { Color::Green }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Default Branch: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&repo.default_branch),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(repo.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("URL: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&repo.url, Style::default().fg(Color::Blue)),
            ]),
        ];
        
        let settings = Paragraph::new(settings_text)
            .block(Block::default().title("Settings").borders(Borders::ALL))
            .style(Style::default());
        
        frame.render_widget(settings, area);
        Ok(())
    }
}