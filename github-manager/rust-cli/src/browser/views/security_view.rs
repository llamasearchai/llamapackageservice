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

use crate::browser::state::{SharedState, ViewType, SecurityIssue};
use crate::browser::views::View;
use crate::error::Result;

pub struct SecurityView {
    state: SharedState,
    selected_tab: usize,
    selected_issue: usize,
    scan_in_progress: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SecurityTab {
    Overview,
    Vulnerabilities,
    Secrets,
    Dependencies,
    Audit,
}

impl SecurityView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_tab: 0,
            selected_issue: 0,
            scan_in_progress: false,
        }
    }
    
    fn get_tabs() -> Vec<&'static str> {
        vec!["Overview", "Vulnerabilities", "Secrets", "Dependencies", "Audit Log"]
    }
    
    fn get_severity_color(severity: &str) -> Color {
        match severity.to_lowercase().as_str() {
            "critical" => Color::Red,
            "high" => Color::LightRed,
            "medium" => Color::Yellow,
            "low" => Color::Blue,
            "info" => Color::Gray,
            _ => Color::White,
        }
    }
    
    fn get_severity_icon(severity: &str) -> &'static str {
        match severity.to_lowercase().as_str() {
            "critical" => "🔴",
            "high" => "🟠",
            "medium" => "🟡",
            "low" => "🔵",
            "info" => "⚪",
            _ => "⚫",
        }
    }
}

#[async_trait]
impl View for SecurityView {
    fn view_type(&self) -> ViewType {
        ViewType::Security
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
            .block(Block::default().borders(Borders::ALL).title("Security Center"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab);
        
        frame.render_widget(tabs, chunks[0]);
        
        // Draw content based on selected tab
        match self.selected_tab {
            0 => self.draw_overview(frame, chunks[1]).await?,
            1 => self.draw_vulnerabilities(frame, chunks[1]).await?,
            2 => self.draw_secrets(frame, chunks[1]).await?,
            3 => self.draw_dependencies(frame, chunks[1]).await?,
            4 => self.draw_audit(frame, chunks[1]).await?,
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
            KeyCode::Char('5') => self.selected_tab = 4,
            KeyCode::Char('s') => {
                self.scan_in_progress = true;
                // Trigger security scan
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let state = self.state.read().await;
                if self.selected_issue < state.security_issues.len().saturating_sub(1) {
                    self.selected_issue += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_issue > 0 {
                    self.selected_issue -= 1;
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

impl SecurityView {
    async fn draw_overview<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Security score
                Constraint::Length(6),  // Summary stats
                Constraint::Min(0),     // Recent issues
            ])
            .split(area);
        
        let state = self.state.read().await;
        
        // Calculate security score
        let security_score = if state.analytics_data.is_empty() {
            85.0 // Default score
        } else {
            state.analytics_data.values()
                .map(|a| a.security_score)
                .sum::<f64>() / state.analytics_data.len() as f64 * 100.0
        };
        
        // Draw security score gauge
        let score_color = if security_score >= 80.0 {
            Color::Green
        } else if security_score >= 60.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        
        let score_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Overall Security Score"))
            .gauge_style(Style::default().fg(score_color))
            .percent(security_score as u16)
            .label(format!("{:.1}%", security_score));
        
        frame.render_widget(score_gauge, chunks[0]);
        
        // Draw summary statistics
        let critical_count = state.security_issues.iter()
            .filter(|i| i.severity.to_lowercase() == "critical")
            .count();
        let high_count = state.security_issues.iter()
            .filter(|i| i.severity.to_lowercase() == "high")
            .count();
        let medium_count = state.security_issues.iter()
            .filter(|i| i.severity.to_lowercase() == "medium")
            .count();
        let low_count = state.security_issues.iter()
            .filter(|i| i.severity.to_lowercase() == "low")
            .count();
        
        let stats = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("🔴 Critical: ", Style::default().fg(Color::Red)),
                Span::styled(critical_count.to_string(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("🟠 High: ", Style::default().fg(Color::LightRed)),
                Span::styled(high_count.to_string(), Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("🟡 Medium: ", Style::default().fg(Color::Yellow)),
                Span::styled(medium_count.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("🔵 Low: ", Style::default().fg(Color::Blue)),
                Span::styled(low_count.to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Last scan: "),
                Span::styled("2 hours ago", Style::default().fg(Color::DarkGray)),
                Span::raw(" | "),
                Span::styled("Press 's' to run new scan", Style::default().fg(Color::DarkGray)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Summary"))
        .alignment(Alignment::Center);
        
        frame.render_widget(stats, chunks[1]);
        
        // Draw recent issues
        let recent_issues: Vec<ListItem> = state.security_issues
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, issue)| {
                let style = if i == self.selected_issue {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", Self::get_severity_icon(&issue.severity)),
                        style,
                    ),
                    Span::styled(
                        format!("[{}] ", issue.severity.to_uppercase()),
                        style.fg(Self::get_severity_color(&issue.severity)),
                    ),
                    Span::styled(&issue.title, style),
                ]))
            })
            .collect();
        
        let issues_list = List::new(recent_issues)
            .block(Block::default().borders(Borders::ALL).title("Recent Security Issues"));
        
        frame.render_widget(issues_list, chunks[2]);
        
        Ok(())
    }
    
    async fn draw_vulnerabilities<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let state = self.state.read().await;
        
        if self.scan_in_progress {
            let scanning = Paragraph::new(vec![
                Line::from(""),
                Line::from("⟳ Scanning for vulnerabilities..."),
                Line::from(""),
                Line::from("This may take a few minutes."),
            ])
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Vulnerability Scanner"));
            
            frame.render_widget(scanning, area);
        } else if state.security_issues.is_empty() {
            let no_issues = Paragraph::new(vec![
                Line::from(""),
                Line::from("✅ No vulnerabilities found!"),
                Line::from(""),
                Line::from("Press 's' to run a security scan"),
            ])
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Vulnerabilities"));
            
            frame.render_widget(no_issues, area);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
            
            // Vulnerability list
            let vulns: Vec<ListItem> = state.security_issues
                .iter()
                .enumerate()
                .map(|(i, issue)| {
                    let style = if i == self.selected_issue {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    
                    let mut lines = vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{} ", Self::get_severity_icon(&issue.severity)),
                                style,
                            ),
                            Span::styled(
                                format!("[{}] ", issue.severity.to_uppercase()),
                                style.fg(Self::get_severity_color(&issue.severity)),
                            ),
                            Span::styled(&issue.title, style.add_modifier(Modifier::BOLD)),
                        ]),
                    ];
                    
                    if let Some(path) = &issue.file_path {
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(
                                format!("📁 {}:{}", path, issue.line_number.unwrap_or(0)),
                                style.fg(Color::DarkGray),
                            ),
                        ]));
                    }
                    
                    ListItem::new(lines)
                })
                .collect();
            
            let vuln_list = List::new(vulns)
                .block(Block::default().borders(Borders::ALL).title("Vulnerabilities"));
            
            frame.render_widget(vuln_list, chunks[0]);
            
            // Selected vulnerability details
            if let Some(issue) = state.security_issues.get(self.selected_issue) {
                let mut details = vec![
                    Line::from(vec![
                        Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&issue.title),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(Span::raw(&issue.description)),
                ];
                
                if let Some(remediation) = &issue.remediation {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("Remediation: ", Style::default().add_modifier(Modifier::BOLD)),
                    ]));
                    details.push(Line::from(Span::styled(
                        remediation,
                        Style::default().fg(Color::Green),
                    )));
                }
                
                let details_widget = Paragraph::new(details)
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .block(Block::default().borders(Borders::ALL).title("Details"));
                
                frame.render_widget(details_widget, chunks[1]);
            }
        }
        
        Ok(())
    }
    
    async fn draw_secrets<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let secrets_found = vec![
            ("API Key", "src/config.rs:45", "HIGH", "github_api_key = \"ghp_...\""),
            ("AWS Secret", ".env:12", "CRITICAL", "AWS_SECRET_ACCESS_KEY=..."),
            ("Database URL", "config/prod.yml:8", "HIGH", "postgres://user:pass@..."),
        ];
        
        let items: Vec<ListItem> = secrets_found
            .iter()
            .map(|(type_name, location, severity, preview)| {
                let severity_color = match *severity {
                    "CRITICAL" => Color::Red,
                    "HIGH" => Color::LightRed,
                    "MEDIUM" => Color::Yellow,
                    _ => Color::Blue,
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", severity),
                            Style::default().fg(severity_color),
                        ),
                        Span::styled(type_name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" found in "),
                        Span::styled(location, Style::default().fg(Color::Blue)),
                    ]),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(preview, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Exposed Secrets"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
    
    async fn draw_dependencies<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let deps = vec![
            ("tokio", "1.35.0", "1.36.0", "LOW", "Minor update available"),
            ("openssl", "0.10.45", "0.10.64", "HIGH", "Security patches available"),
            ("serde", "1.0.195", "1.0.196", "INFO", "Patch update available"),
            ("actix-web", "4.3.0", "4.5.0", "MEDIUM", "Breaking changes in v4.4+"),
        ];
        
        let items: Vec<ListItem> = deps
            .iter()
            .map(|(name, current, latest, severity, note)| {
                let severity_color = match *severity {
                    "HIGH" => Color::Red,
                    "MEDIUM" => Color::Yellow,
                    "LOW" => Color::Blue,
                    _ => Color::Gray,
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(current),
                        Span::raw(" → "),
                        Span::styled(latest, Style::default().fg(Color::Green)),
                        Span::raw(" "),
                        Span::styled(
                            format!("[{}]", severity),
                            Style::default().fg(severity_color),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(note, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Dependency Security"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
    
    async fn draw_audit<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let audit_events = vec![
            ("2m ago", "🔍 Security scan completed", "system", Color::Green),
            ("15m ago", "🔐 Access granted to production DB", "user123", Color::Yellow),
            ("1h ago", "🚨 Failed login attempt", "unknown", Color::Red),
            ("2h ago", "📝 Security policy updated", "admin", Color::Blue),
            ("3h ago", "🔑 API key rotated", "system", Color::Green),
            ("5h ago", "🛡️ Firewall rules updated", "admin", Color::Blue),
        ];
        
        let items: Vec<ListItem> = audit_events
            .iter()
            .map(|(time, event, user, color)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<8}", time), Style::default().fg(Color::DarkGray)),
                    Span::styled(event, Style::default().fg(*color)),
                    Span::raw(" by "),
                    Span::styled(user, Style::default().add_modifier(Modifier::ITALIC)),
                ]))
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Security Audit Log"));
        
        frame.render_widget(list, area);
        
        Ok(())
    }
}