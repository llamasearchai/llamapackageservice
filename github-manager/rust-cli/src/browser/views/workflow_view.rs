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
use chrono::{DateTime, Utc};

use crate::browser::state::{SharedState, ViewType, WorkflowRun};
use crate::browser::views::View;
use crate::error::Result;
use crate::github::Workflow;

pub struct WorkflowView {
    state: SharedState,
    selected_tab: usize,
    selected_workflow: usize,
    selected_run: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WorkflowTab {
    Active,
    History,
    Templates,
    Schedule,
}

impl WorkflowView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_tab: 0,
            selected_workflow: 0,
            selected_run: 0,
        }
    }
    
    fn get_tabs() -> Vec<&'static str> {
        vec!["Active Workflows", "History", "Templates", "Schedule"]
    }
    
    fn get_status_color(status: &str) -> Color {
        match status.to_lowercase().as_str() {
            "success" | "completed" => Color::Green,
            "failure" | "failed" => Color::Red,
            "in_progress" | "running" | "queued" => Color::Yellow,
            "cancelled" | "skipped" => Color::DarkGray,
            _ => Color::White,
        }
    }
    
    fn get_status_icon(status: &str) -> &'static str {
        match status.to_lowercase().as_str() {
            "success" | "completed" => "✅",
            "failure" | "failed" => "❌",
            "in_progress" | "running" => "🔄",
            "queued" => "⏳",
            "cancelled" => "⏹️",
            "skipped" => "⏭️",
            _ => "❓",
        }
    }
}

#[async_trait]
impl View for WorkflowView {
    fn view_type(&self) -> ViewType {
        ViewType::Workflow
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
            ])
            .split(area);
        
        // Draw tabs
        let tabs = Tabs::new(Self::get_tabs())
            .block(Block::default().borders(Borders::ALL).title("Workflows"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab);
        
        frame.render_widget(tabs, chunks[0]);
        
        // Draw content based on selected tab
        match self.selected_tab {
            0 => self.draw_active_workflows(frame, chunks[1]).await?,
            1 => self.draw_history(frame, chunks[1]).await?,
            2 => self.draw_templates(frame, chunks[1]).await?,
            3 => self.draw_schedule(frame, chunks[1]).await?,
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Tab => {
                self.selected_tab = (self.selected_tab + 1) % Self::get_tabs().len();
            }
            KeyCode::BackTab => {
                if self.selected_tab > 0 {
                    self.selected_tab -= 1;
                } else {
                    self.selected_tab = Self::get_tabs().len() - 1;
                }
            }
            KeyCode::Char('1') => self.selected_tab = 0,
            KeyCode::Char('2') => self.selected_tab = 1,
            KeyCode::Char('3') => self.selected_tab = 2,
            KeyCode::Char('4') => self.selected_tab = 3,
            KeyCode::Char('j') | KeyCode::Down => {
                match self.selected_tab {
                    0 => {
                        let state = self.state.read().await;
                        if self.selected_workflow < state.workflows.len().saturating_sub(1) {
                            self.selected_workflow += 1;
                        }
                    }
                    1 => {
                        self.selected_run += 1;
                    }
                    _ => {}
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.selected_tab {
                    0 => {
                        if self.selected_workflow > 0 {
                            self.selected_workflow -= 1;
                        }
                    }
                    1 => {
                        if self.selected_run > 0 {
                            self.selected_run -= 1;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                // Trigger workflow or view details
            }
            KeyCode::Char('r') => {
                // Re-run workflow
            }
            KeyCode::Char('c') => {
                // Cancel workflow
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                let mut state = self.state.write().await;
                state.navigate_back();
            }
            _ => {}
        }
        
        Ok(false)
    }
}

impl WorkflowView {
    async fn draw_active_workflows<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let state = self.state.read().await;
        
        if state.workflows.is_empty() {
            let no_workflows = Paragraph::new(vec![
                Line::from(""),
                Line::from("No active workflows"),
                Line::from(""),
                Line::from("Press 'Enter' to create a new workflow"),
            ])
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
            
            frame.render_widget(no_workflows, area);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
            
            // Workflow list
            let workflows: Vec<ListItem> = state.workflows
                .iter()
                .enumerate()
                .map(|(i, workflow)| {
                    let style = if i == self.selected_workflow {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    
                    // Mock status - in real implementation would come from workflow runs
                    let status = "success";
                    
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{} ", Self::get_status_icon(status)),
                                style,
                            ),
                            Span::styled(&workflow.name, style.add_modifier(Modifier::BOLD)),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(
                                format!("Repo: {} | ", workflow.repository),
                                style.fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("Trigger: {}", workflow.trigger_type),
                                style.fg(Color::DarkGray),
                            ),
                        ]),
                    ])
                })
                .collect();
            
            let workflow_list = List::new(workflows)
                .block(Block::default().borders(Borders::ALL).title("Workflows"));
            
            frame.render_widget(workflow_list, chunks[0]);
            
            // Selected workflow details
            if let Some(workflow) = state.workflows.get(self.selected_workflow) {
                let details = self.draw_workflow_details(workflow, chunks[1]);
                frame.render_widget(details, chunks[1]);
            }
        }
        
        Ok(())
    }
    
    fn draw_workflow_details(&self, workflow: &Workflow, area: Rect) -> Paragraph {
        let details = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Repository: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&workflow.repository, Style::default().fg(Color::Blue)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Trigger: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.trigger_type),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Last Run: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("2 hours ago"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions:", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(": Run now"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(": Re-run last"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("c", Style::default().fg(Color::Red)),
                Span::raw(": Cancel"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("e", Style::default().fg(Color::Blue)),
                Span::raw(": Edit"),
            ]),
        ];
        
        Paragraph::new(details)
            .block(Block::default().borders(Borders::ALL).title("Details"))
    }
    
    async fn draw_history<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let history_runs = vec![
            ("CI/CD Pipeline", "llamaagent", "success", "2m ago", 156),
            ("Security Scan", "llamagraph", "failure", "15m ago", 89),
            ("Deploy to Staging", "llama-cli", "success", "1h ago", 234),
            ("Unit Tests", "OpenPersona", "success", "2h ago", 45),
            ("Code Quality Check", "llama-metasearch", "success", "3h ago", 78),
            ("Dependency Update", "llamaagent", "cancelled", "5h ago", 12),
        ];
        
        let items: Vec<ListItem> = history_runs
            .iter()
            .enumerate()
            .map(|(i, (name, repo, status, time, duration))| {
                let style = if i == self.selected_run {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", Self::get_status_icon(status)),
                            style,
                        ),
                        Span::styled(name, style.add_modifier(Modifier::BOLD)),
                        Span::raw(" - "),
                        Span::styled(repo, style.fg(Color::Blue)),
                    ]),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(
                            format!("⏱️  {} ({} seconds)", time, duration),
                            style.fg(Color::DarkGray),
                        ),
                    ]),
                ])
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Workflow History"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
    
    async fn draw_templates<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let templates = vec![
            ("Security Audit", "Complete security scan with vulnerability detection", "security"),
            ("Performance Test", "Run performance benchmarks and profiling", "performance"),
            ("Release Pipeline", "Build, test, and deploy to production", "deployment"),
            ("Code Quality", "Linting, formatting, and complexity analysis", "quality"),
            ("Dependency Update", "Update and test all dependencies", "maintenance"),
            ("Documentation Build", "Generate and deploy documentation", "docs"),
        ];
        
        let items: Vec<ListItem> = templates
            .iter()
            .map(|(name, description, category)| {
                let category_color = match *category {
                    "security" => Color::Red,
                    "performance" => Color::Yellow,
                    "deployment" => Color::Green,
                    "quality" => Color::Blue,
                    "maintenance" => Color::Magenta,
                    "docs" => Color::Cyan,
                    _ => Color::White,
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", category),
                            Style::default().fg(category_color),
                        ),
                        Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(description, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Workflow Templates"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
    
    async fn draw_schedule<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let scheduled_workflows = vec![
            ("Daily Security Scan", "0 0 * * *", "Every day at midnight", true),
            ("Weekly Dependency Update", "0 0 * * 1", "Every Monday at midnight", true),
            ("Monthly Performance Report", "0 0 1 * *", "First day of month", false),
            ("Hourly Health Check", "0 * * * *", "Every hour", true),
        ];
        
        let items: Vec<ListItem> = scheduled_workflows
            .iter()
            .map(|(name, cron, description, enabled)| {
                let status_icon = if *enabled { "🟢" } else { "🔴" };
                let status_text = if *enabled { "Enabled" } else { "Disabled" };
                let status_color = if *enabled { Color::Green } else { Color::Red };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::raw(format!("{} ", status_icon)),
                        Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" - "),
                        Span::styled(status_text, Style::default().fg(status_color)),
                    ]),
                    Line::from(vec![
                        Span::raw("   📅 "),
                        Span::styled(cron, Style::default().fg(Color::Yellow)),
                        Span::raw(" - "),
                        Span::styled(description, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Scheduled Workflows"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
}