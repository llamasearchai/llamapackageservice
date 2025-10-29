use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, BarChart, Block, Borders, Chart, Dataset, Gauge, List, ListItem, Paragraph, Sparkline,
    },
    Frame,
};

use crate::browser::state::{SharedState, ViewType, AnalyticsData};
use crate::browser::views::View;
use crate::error::Result;

pub struct AnalyticsView {
    state: SharedState,
    selected_tab: usize,
    selected_repo: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AnalyticsTab {
    Overview,
    CodeQuality,
    Activity,
    Languages,
}

impl AnalyticsView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_tab: 0,
            selected_repo: 0,
        }
    }
    
    fn get_tabs() -> Vec<&'static str> {
        vec!["Overview", "Code Quality", "Activity", "Languages"]
    }
}

#[async_trait]
impl View for AnalyticsView {
    fn view_type(&self) -> ViewType {
        ViewType::Analytics
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
            ])
            .split(area);
        
        // Draw header with tabs
        let tabs: Vec<Span> = Self::get_tabs()
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                if i == self.selected_tab {
                    Span::styled(
                        format!(" {} ", tab),
                        Style::default().bg(Color::Blue).fg(Color::White),
                    )
                } else {
                    Span::styled(format!(" {} ", tab), Style::default())
                }
            })
            .collect();
        
        let header = Paragraph::new(Line::from(tabs))
            .block(Block::default().borders(Borders::ALL).title("Analytics"))
            .alignment(Alignment::Center);
        
        frame.render_widget(header, chunks[0]);
        
        // Draw content based on selected tab
        match self.selected_tab {
            0 => self.draw_overview(frame, chunks[1]).await?,
            1 => self.draw_code_quality(frame, chunks[1]).await?,
            2 => self.draw_activity(frame, chunks[1]).await?,
            3 => self.draw_languages(frame, chunks[1]).await?,
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
                let state = self.state.read().await;
                if self.selected_repo < state.repositories.len().saturating_sub(1) {
                    self.selected_repo += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_repo > 0 {
                    self.selected_repo -= 1;
                }
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

impl AnalyticsView {
    async fn draw_overview<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Summary cards
                Constraint::Min(0),     // Repository metrics
            ])
            .split(area);
        
        // Draw summary cards
        let card_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);
        
        let state = self.state.read().await;
        
        // Calculate aggregate metrics
        let total_repos = state.repositories.len();
        let total_stars: u32 = state.repositories.iter().map(|r| r.stars).sum();
        let total_issues: u32 = state.repositories.iter().map(|r| r.open_issues_count).sum();
        let avg_quality = if state.analytics_data.is_empty() {
            0.0
        } else {
            state.analytics_data.values()
                .map(|a| a.code_quality_score)
                .sum::<f64>() / state.analytics_data.len() as f64
        };
        
        // Draw metric cards
        let metrics = [
            ("Total Repositories", total_repos.to_string(), Color::Blue),
            ("Total Stars", format!("⭐ {}", total_stars), Color::Yellow),
            ("Open Issues", total_issues.to_string(), Color::Red),
            ("Avg Quality", format!("{:.1}%", avg_quality * 100.0), Color::Green),
        ];
        
        for (i, (label, value, color)) in metrics.iter().enumerate() {
            let card = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    value,
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(label, Style::default())),
            ])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
            
            frame.render_widget(card, card_chunks[i]);
        }
        
        // Draw repository metrics table
        let header = vec!["Repository", "Quality", "Coverage", "Security", "Activity"];
        let rows: Vec<Vec<String>> = state
            .repositories
            .iter()
            .map(|repo| {
                if let Some(analytics) = state.analytics_data.get(&repo.name) {
                    vec![
                        repo.name.clone(),
                        format!("{:.0}%", analytics.code_quality_score * 100.0),
                        format!("{:.0}%", analytics.test_coverage * 100.0),
                        format!("{:.0}%", analytics.security_score * 100.0),
                        format!("{} issues, {} PRs", analytics.open_issues, analytics.open_prs),
                    ]
                } else {
                    vec![
                        repo.name.clone(),
                        "N/A".to_string(),
                        "N/A".to_string(),
                        "N/A".to_string(),
                        format!("{} issues", repo.open_issues_count),
                    ]
                }
            })
            .collect();
        
        let selected = self.selected_repo;
        let items: Vec<ListItem> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let style = if i == selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                
                ListItem::new(Line::from(
                    row.iter()
                        .enumerate()
                        .map(|(j, cell)| {
                            let width = match j {
                                0 => 25,
                                1..=3 => 10,
                                _ => 20,
                            };
                            Span::styled(
                                format!("{:<width$}", cell, width = width),
                                style,
                            )
                        })
                        .collect::<Vec<_>>(),
                ))
            })
            .collect();
        
        let table_header = Line::from(
            header
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let width = match i {
                        0 => 25,
                        1..=3 => 10,
                        _ => 20,
                    };
                    Span::styled(
                        format!("{:<width$}", h, width = width),
                        Style::default().add_modifier(Modifier::BOLD),
                    )
                })
                .collect::<Vec<_>>(),
        );
        
        let mut all_items = vec![ListItem::new(table_header)];
        all_items.extend(items);
        
        let list = List::new(all_items)
            .block(Block::default().borders(Borders::ALL).title("Repository Metrics"));
        
        frame.render_widget(list, chunks[1]);
        
        Ok(())
    }
    
    async fn draw_code_quality<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        
        let state = self.state.read().await;
        
        // Quality scores chart
        let quality_data: Vec<(&str, u64)> = state
            .repositories
            .iter()
            .take(10)
            .map(|repo| {
                let score = state
                    .analytics_data
                    .get(&repo.name)
                    .map(|a| (a.code_quality_score * 100.0) as u64)
                    .unwrap_or(0);
                (repo.name.as_str(), score)
            })
            .collect();
        
        let quality_chart = BarChart::default()
            .block(Block::default().borders(Borders::ALL).title("Code Quality Scores"))
            .data(&quality_data)
            .bar_width(3)
            .bar_gap(1)
            .value_style(Style::default().fg(Color::Green))
            .label_style(Style::default().fg(Color::Gray))
            .bar_style(Style::default().fg(Color::Green));
        
        frame.render_widget(quality_chart, chunks[0]);
        
        // Test coverage
        let coverage_data: Vec<(&str, u64)> = state
            .repositories
            .iter()
            .take(10)
            .map(|repo| {
                let coverage = state
                    .analytics_data
                    .get(&repo.name)
                    .map(|a| (a.test_coverage * 100.0) as u64)
                    .unwrap_or(0);
                (repo.name.as_str(), coverage)
            })
            .collect();
        
        let coverage_chart = BarChart::default()
            .block(Block::default().borders(Borders::ALL).title("Test Coverage"))
            .data(&coverage_data)
            .bar_width(3)
            .bar_gap(1)
            .value_style(Style::default().fg(Color::Blue))
            .label_style(Style::default().fg(Color::Gray))
            .bar_style(Style::default().fg(Color::Blue));
        
        frame.render_widget(coverage_chart, chunks[1]);
        
        Ok(())
    }
    
    async fn draw_activity<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Activity chart
                Constraint::Percentage(40), // Recent events
            ])
            .split(area);
        
        // Mock activity data (commits per day for last 30 days)
        let activity_data: Vec<(f64, f64)> = (0..30)
            .map(|i| {
                let x = i as f64;
                // Simple sine wave with some variance
                let y = ((i as f64 * 0.2).sin() + 1.0) * 10.0 + (i % 7) as f64;
                (x, y)
            })
            .collect();
        
        let datasets = vec![Dataset::default()
            .name("Commits")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(&activity_data)];
        
        let chart = Chart::new(datasets)
            .block(Block::default().borders(Borders::ALL).title("Activity (Last 30 Days)"))
            .x_axis(
                Axis::default()
                    .title("Days Ago")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 30.0])
                    .labels(vec!["30", "20", "10", "0"]),
            )
            .y_axis(
                Axis::default()
                    .title("Commits")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 20.0])
                    .labels(vec!["0", "10", "20"]),
            );
        
        frame.render_widget(chart, chunks[0]);
        
        // Recent events
        let events = vec![
            ("2m ago", "🔀 Merged PR #456 in llamaagent", Color::Green),
            ("15m ago", "💬 New issue #789 in llamagraph", Color::Yellow),
            ("1h ago", "⚡ Push to main in llama-cli", Color::Blue),
            ("2h ago", "🏷️ New release v1.2.0 in OpenPersona", Color::Magenta),
            ("3h ago", "🔧 CI passed for PR #123", Color::Green),
        ];
        
        let event_items: Vec<ListItem> = events
            .iter()
            .map(|(time, event, color)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<10}", time), Style::default().fg(Color::DarkGray)),
                    Span::styled(event, Style::default().fg(*color)),
                ]))
            })
            .collect();
        
        let events_list = List::new(event_items)
            .block(Block::default().borders(Borders::ALL).title("Recent Activity"));
        
        frame.render_widget(events_list, chunks[1]);
        
        Ok(())
    }
    
    async fn draw_languages<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let state = self.state.read().await;
        
        // Aggregate language statistics
        let mut language_totals: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        
        for analytics in state.analytics_data.values() {
            for (lang, percentage) in &analytics.languages {
                *language_totals.entry(lang.clone()).or_insert(0.0) += percentage;
            }
        }
        
        let mut languages: Vec<(String, f64)> = language_totals.into_iter().collect();
        languages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        
        // Language distribution chart
        let lang_data: Vec<(&str, u64)> = languages
            .iter()
            .take(8)
            .map(|(lang, val)| (lang.as_str(), *val as u64))
            .collect();
        
        let lang_chart = BarChart::default()
            .block(Block::default().borders(Borders::ALL).title("Language Distribution"))
            .data(&lang_data)
            .bar_width(5)
            .bar_gap(2)
            .value_style(Style::default().fg(Color::Cyan))
            .label_style(Style::default().fg(Color::Gray))
            .bar_style(Style::default().fg(Color::Cyan));
        
        frame.render_widget(lang_chart, chunks[0]);
        
        // Language details
        let lang_items: Vec<ListItem> = languages
            .iter()
            .enumerate()
            .map(|(i, (lang, percentage))| {
                let color = match i % 6 {
                    0 => Color::Blue,
                    1 => Color::Green,
                    2 => Color::Yellow,
                    3 => Color::Magenta,
                    4 => Color::Cyan,
                    _ => Color::Red,
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:<15}", lang),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{:>6.1}%", percentage)),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            "█".repeat((percentage / 5.0) as usize),
                            Style::default().fg(color),
                        ),
                    ]),
                ])
            })
            .collect();
        
        let lang_list = List::new(lang_items)
            .block(Block::default().borders(Borders::ALL).title("Language Breakdown"));
        
        frame.render_widget(lang_list, chunks[1]);
        
        Ok(())
    }
}