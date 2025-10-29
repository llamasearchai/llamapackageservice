use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::browser::state::{SharedState, ViewType};
use crate::browser::views::View;
use crate::error::Result;

pub struct HelpView {
    selected_tab: usize,
    scroll_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HelpTab {
    General,
    Navigation,
    Views,
    Shortcuts,
    About,
}

impl HelpView {
    pub fn new() -> Self {
        Self {
            selected_tab: 0,
            scroll_offset: 0,
        }
    }
    
    fn get_tabs() -> Vec<&'static str> {
        vec!["General", "Navigation", "Views", "Shortcuts", "About"]
    }
}

#[async_trait]
impl View for HelpView {
    fn view_type(&self) -> ViewType {
        ViewType::Help
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(area);
        
        // Draw tabs
        let tabs = Tabs::new(Self::get_tabs())
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab);
        
        frame.render_widget(tabs, chunks[0]);
        
        // Draw content based on selected tab
        match self.selected_tab {
            0 => self.draw_general_help(frame, chunks[1])?,
            1 => self.draw_navigation_help(frame, chunks[1])?,
            2 => self.draw_views_help(frame, chunks[1])?,
            3 => self.draw_shortcuts_help(frame, chunks[1])?,
            4 => self.draw_about(frame, chunks[1])?,
            _ => {}
        }
        
        // Draw footer
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Tab/BackTab", Style::default().fg(Color::Yellow)),
            Span::raw(": Switch tabs | "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(": Scroll | "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(": Close help"),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(footer, chunks[2]);
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Tab => {
                self.selected_tab = (self.selected_tab + 1) % Self::get_tabs().len();
                self.scroll_offset = 0;
            }
            KeyCode::BackTab => {
                if self.selected_tab > 0 {
                    self.selected_tab -= 1;
                } else {
                    self.selected_tab = Self::get_tabs().len() - 1;
                }
                self.scroll_offset = 0;
            }
            KeyCode::Char('1') => { self.selected_tab = 0; self.scroll_offset = 0; }
            KeyCode::Char('2') => { self.selected_tab = 1; self.scroll_offset = 0; }
            KeyCode::Char('3') => { self.selected_tab = 2; self.scroll_offset = 0; }
            KeyCode::Char('4') => { self.selected_tab = 3; self.scroll_offset = 0; }
            KeyCode::Char('5') => { self.selected_tab = 4; self.scroll_offset = 0; }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset += 1;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // This would typically close the help view
                // But since we don't have direct access to state here, we return false
                return Ok(false);
            }
            _ => {}
        }
        
        Ok(false)
    }
}

impl HelpView {
    fn draw_general_help<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let help_text = vec![
            Line::from(Span::styled(
                "LlamaSearchAI Repository Manager",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Welcome to the advanced GitHub repository management tool for LlamaSearchAI!"),
            Line::from(""),
            Line::from(Span::styled("Key Features:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  • Browse and manage all repositories in your organization"),
            Line::from("  • View and edit code with syntax highlighting"),
            Line::from("  • Search across all repositories with advanced filters"),
            Line::from("  • Monitor repository analytics and health metrics"),
            Line::from("  • Run security scans and vulnerability assessments"),
            Line::from("  • Execute and monitor workflows"),
            Line::from("  • Generate concatenated files for analysis"),
            Line::from(""),
            Line::from(Span::styled("Getting Started:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  1. Navigate through repositories using arrow keys"),
            Line::from("  2. Press Enter to view repository details"),
            Line::from("  3. Use number keys to quickly switch between views"),
            Line::from("  4. Press '/' to search in most views"),
            Line::from("  5. Press Ctrl+H anytime to access this help"),
            Line::from(""),
            Line::from(Span::styled("Tips:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  • Use Tab to cycle through UI elements"),
            Line::from("  • Most lists support vim-style navigation (j/k)"),
            Line::from("  • Press 'g' to go to top, 'G' to go to bottom"),
            Line::from("  • Many views have context-specific shortcuts"),
        ];
        
        let paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL))
            .scroll((self.scroll_offset as u16, 0));
        
        frame.render_widget(paragraph, area);
        Ok(())
    }
    
    fn draw_navigation_help<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let nav_items = vec![
            ("Global Navigation", vec![
                ("Ctrl+Q", "Quit application"),
                ("Ctrl+H", "Show this help"),
                ("Ctrl+S", "Global search"),
                ("Ctrl+R", "Refresh current view"),
                ("Esc", "Go back / Cancel"),
                ("Alt+1-9", "Switch to view by number"),
            ]),
            ("List Navigation", vec![
                ("j/↓", "Move down"),
                ("k/↑", "Move up"),
                ("g", "Go to top"),
                ("G", "Go to bottom"),
                ("Enter", "Select/Open"),
                ("/", "Filter/Search"),
            ]),
            ("View Navigation", vec![
                ("Tab", "Next element"),
                ("Shift+Tab", "Previous element"),
                ("Space", "Toggle/Expand"),
                ("q", "Close view"),
            ]),
        ];
        
        let mut y = 0;
        for (category, shortcuts) in nav_items {
            if y + shortcuts.len() + 2 > area.height as usize {
                break;
            }
            
            let category_widget = Paragraph::new(Span::styled(
                category,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            frame.render_widget(category_widget, Rect::new(area.x, area.y + y as u16, area.width, 1));
            y += 2;
            
            for (key, desc) in shortcuts {
                if y >= area.height as usize {
                    break;
                }
                
                let item = Paragraph::new(Line::from(vec![
                    Span::styled(format!("  {:<12}", key), Style::default().fg(Color::Green)),
                    Span::raw(desc),
                ]));
                frame.render_widget(item, Rect::new(area.x, area.y + y as u16, area.width, 1));
                y += 1;
            }
            y += 1;
        }
        
        Ok(())
    }
    
    fn draw_views_help<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let views = vec![
            ("Repository List (Alt+1)", vec![
                "Browse all repositories in the organization",
                "Sort by name, stars, update time, or language",
                "Filter repositories by name or description",
                "Quick access with number keys",
            ]),
            ("Repository Detail (Alt+2)", vec![
                "View comprehensive repository information",
                "Access files, issues, PRs, and actions",
                "Monitor repository statistics",
                "Navigate with Tab between sections",
            ]),
            ("File Explorer (Alt+3)", vec![
                "Browse repository file structure",
                "Expand/collapse directories",
                "Open files in code viewer",
                "Navigate with arrow keys or h/j/k/l",
            ]),
            ("Code View (Alt+4)", vec![
                "View files with syntax highlighting",
                "Search within files with '/'",
                "Navigate search results with n/N",
                "Scroll horizontally with h/l",
            ]),
            ("Search (Alt+5)", vec![
                "Search across all repositories",
                "Use regex and case-sensitive options",
                "Filter by file types",
                "Jump to results in code view",
            ]),
            ("Analytics (Alt+6)", vec![
                "Monitor repository health metrics",
                "View code quality and test coverage",
                "Track activity and contributions",
                "Analyze language distribution",
            ]),
            ("Workflows (Alt+7)", vec![
                "Manage GitHub Actions workflows",
                "View workflow history and status",
                "Use workflow templates",
                "Schedule automated tasks",
            ]),
            ("Security (Alt+8)", vec![
                "Run security scans",
                "Check for vulnerabilities",
                "Detect exposed secrets",
                "Monitor dependency security",
            ]),
        ];
        
        let items: Vec<ListItem> = views
            .iter()
            .skip(self.scroll_offset)
            .take(area.height as usize / 6)
            .map(|(title, features)| {
                let mut lines = vec![
                    Line::from(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                ];
                for feature in features {
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::raw(*feature),
                    ]));
                }
                lines.push(Line::from(""));
                ListItem::new(lines)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(list, area);
        Ok(())
    }
    
    fn draw_shortcuts_help<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let shortcuts = vec![
            ("Repository List", vec![
                ("s/S", "Sort by stars (shift to reverse)"),
                ("n/N", "Sort by name (shift to reverse)"),
                ("u/U", "Sort by update time (shift to reverse)"),
                ("l/L", "Sort by language (shift to reverse)"),
            ]),
            ("File Explorer", vec![
                ("Space", "Toggle directory expansion"),
                ("h/←", "Collapse directory or go to parent"),
                ("l/→", "Expand directory"),
                ("Enter", "Open file or toggle directory"),
            ]),
            ("Code View", vec![
                ("0", "Go to beginning of line"),
                ("$", "Go to end of line"),
                ("PageUp/PageDown", "Scroll by page"),
                ("/", "Search in file"),
                ("n/N", "Next/Previous search result"),
            ]),
            ("Search View", vec![
                ("c", "Toggle case sensitivity"),
                ("r", "Toggle regex mode"),
                ("w", "Toggle whole word search"),
                ("t", "Filter by file type"),
            ]),
            ("Security View", vec![
                ("s", "Run security scan"),
                ("Tab", "Switch between security tabs"),
                ("Enter", "View issue details"),
            ]),
        ];
        
        let mut content = Vec::new();
        for (view, keys) in shortcuts.iter().skip(self.scroll_offset) {
            content.push(Line::from(Span::styled(
                *view,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            content.push(Line::from(""));
            
            for (key, desc) in keys {
                content.push(Line::from(vec![
                    Span::styled(format!("  {:<10}", key), Style::default().fg(Color::Green)),
                    Span::raw(*desc),
                ]));
            }
            content.push(Line::from(""));
        }
        
        let paragraph = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(paragraph, area);
        Ok(())
    }
    
    fn draw_about<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let about_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "LlamaSearchAI Repository Manager",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Version 1.0.0",
                Style::default().fg(Color::Green),
            )),
            Line::from(""),
            Line::from("A comprehensive GitHub repository management tool"),
            Line::from("designed specifically for the LlamaSearchAI ecosystem."),
            Line::from(""),
            Line::from(Span::styled("Features:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  • Real-time repository synchronization"),
            Line::from("  • Advanced code search with AI integration"),
            Line::from("  • Automated security scanning"),
            Line::from("  • Performance analytics and monitoring"),
            Line::from("  • Workflow automation and management"),
            Line::from("  • Multi-format export capabilities"),
            Line::from(""),
            Line::from(Span::styled("Built with:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  • Rust for performance and reliability"),
            Line::from("  • Ratatui for terminal UI"),
            Line::from("  • Octocrab for GitHub API"),
            Line::from("  • Syntect for syntax highlighting"),
            Line::from(""),
            Line::from(Span::styled("Credits:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from("  LlamaSearchAI Development Team"),
            Line::from(""),
            Line::from(Span::styled(
                "© 2024 LlamaSearchAI. All rights reserved.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        
        let paragraph = Paragraph::new(about_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
        
        frame.render_widget(paragraph, area);
        Ok(())
    }
}