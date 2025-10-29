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
use std::path::PathBuf;

use crate::browser::state::{SharedState, ViewType, Mode, FileNode};
use crate::browser::views::View;
use crate::error::Result;

pub struct FileExplorerView {
    state: SharedState,
    current_path: PathBuf,
    selected_index: usize,
    expanded_dirs: Vec<PathBuf>,
}

impl FileExplorerView {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            current_path: PathBuf::from("/"),
            selected_index: 0,
            expanded_dirs: Vec::new(),
        }
    }
    
    fn flatten_tree(&self, node: &FileNode, depth: usize, items: &mut Vec<(FileNode, usize)>) {
        items.push((node.clone(), depth));
        
        if node.is_dir && self.expanded_dirs.contains(&node.path) {
            for child in &node.children {
                self.flatten_tree(child, depth + 1, items);
            }
        }
    }
    
    fn get_file_icon(node: &FileNode) -> &'static str {
        if node.is_dir {
            if self.expanded_dirs.contains(&node.path) {
                "📂"
            } else {
                "📁"
            }
        } else {
            match node.path.extension().and_then(|e| e.to_str()) {
                Some("rs") => "🦀",
                Some("py") => "🐍",
                Some("js") | Some("jsx") => "📜",
                Some("ts") | Some("tsx") => "📘",
                Some("json") => "📋",
                Some("toml") | Some("yaml") | Some("yml") => "⚙️",
                Some("md") => "📝",
                Some("txt") => "📄",
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") => "🖼️",
                Some("zip") | Some("tar") | Some("gz") => "📦",
                _ => "📄",
            }
        }
    }
}

#[async_trait]
impl View for FileExplorerView {
    fn view_type(&self) -> ViewType {
        ViewType::FileExplorer
    }
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Path bar
                Constraint::Min(0),    // File tree
                Constraint::Length(3), // Info bar
            ])
            .split(area);
        
        let state = self.state.read().await;
        
        // Draw path bar
        let path_text = if let Some(repo) = &state.selected_repository {
            format!("{} / {}", repo.name, self.current_path.display())
        } else {
            self.current_path.display().to_string()
        };
        
        let path_bar = Paragraph::new(Line::from(vec![
            Span::raw("📍 "),
            Span::styled(path_text, Style::default().add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Path"));
        
        frame.render_widget(path_bar, chunks[0]);
        
        // Draw file tree
        if let Some(root) = &state.repository_files {
            let mut items = Vec::new();
            self.flatten_tree(root, 0, &mut items);
            
            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(i, (node, depth))| {
                    let indent = "  ".repeat(*depth);
                    let icon = Self::get_file_icon(node);
                    let name = node.name.clone();
                    
                    let style = if i == self.selected_index {
                        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                    } else if node.is_dir {
                        Style::default().fg(Color::Blue)
                    } else {
                        Style::default()
                    };
                    
                    let mut spans = vec![
                        Span::raw(indent),
                        Span::raw(format!("{} ", icon)),
                        Span::styled(name, style),
                    ];
                    
                    if let Some(size) = node.size {
                        spans.push(Span::styled(
                            format!(" ({})", crate::utils::format_file_size(size)),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                    
                    ListItem::new(Line::from(spans))
                })
                .collect();
            
            let file_list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title("Files"))
                .style(Style::default());
            
            frame.render_widget(file_list, chunks[1]);
        } else {
            let loading = Paragraph::new("Loading file tree...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Files"));
            frame.render_widget(loading, chunks[1]);
        }
        
        // Draw info bar
        let info_text = if let Some(root) = &state.repository_files {
            let items = Vec::new();
            let mut flattened = Vec::new();
            self.flatten_tree(root, 0, &mut flattened);
            
            if let Some((node, _)) = flattened.get(self.selected_index) {
                if node.is_dir {
                    format!("Directory: {} items", node.children.len())
                } else if let Some(modified) = &node.modified {
                    format!("Modified: {}", modified.format("%Y-%m-%d %H:%M:%S"))
                } else {
                    "File".to_string()
                }
            } else {
                "No selection".to_string()
            }
        } else {
            "No files loaded".to_string()
        };
        
        let info_bar = Paragraph::new(Line::from(vec![
            Span::raw("ℹ️  "),
            Span::raw(info_text),
            Span::raw(" | "),
            Span::styled("Enter: Open | Space: Toggle | q: Back", Style::default().fg(Color::DarkGray)),
        ]))
        .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(info_bar, chunks[2]);
        
        Ok(())
    }
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        let mut state = self.state.write().await;
        
        match state.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            if self.selected_index < items.len().saturating_sub(1) {
                                self.selected_index += 1;
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Char('g') => {
                        self.selected_index = 0;
                    }
                    KeyCode::Char('G') => {
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            if !items.is_empty() {
                                self.selected_index = items.len() - 1;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            
                            if let Some((node, _)) = items.get(self.selected_index) {
                                if node.is_dir {
                                    // Toggle directory expansion
                                    if let Some(pos) = self.expanded_dirs.iter().position(|p| p == &node.path) {
                                        self.expanded_dirs.remove(pos);
                                    } else {
                                        self.expanded_dirs.push(node.path.clone());
                                    }
                                } else {
                                    // Open file in code view
                                    state.current_file_path = Some(node.path.clone());
                                    state.navigate_to(ViewType::Code);
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        // Toggle directory with space
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            
                            if let Some((node, _)) = items.get(self.selected_index) {
                                if node.is_dir {
                                    if let Some(pos) = self.expanded_dirs.iter().position(|p| p == &node.path) {
                                        self.expanded_dirs.remove(pos);
                                    } else {
                                        self.expanded_dirs.push(node.path.clone());
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        // Collapse current directory or go to parent
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            
                            if let Some((node, depth)) = items.get(self.selected_index) {
                                if node.is_dir && self.expanded_dirs.contains(&node.path) {
                                    // Collapse current directory
                                    self.expanded_dirs.retain(|p| p != &node.path);
                                } else if *depth > 0 {
                                    // Go to parent directory
                                    for i in (0..self.selected_index).rev() {
                                        if let Some((_, d)) = items.get(i) {
                                            if *d < *depth {
                                                self.selected_index = i;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        // Expand directory
                        if let Some(root) = &state.repository_files {
                            let mut items = Vec::new();
                            self.flatten_tree(root, 0, &mut items);
                            
                            if let Some((node, _)) = items.get(self.selected_index) {
                                if node.is_dir && !self.expanded_dirs.contains(&node.path) {
                                    self.expanded_dirs.push(node.path.clone());
                                }
                            }
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

impl FileExplorerView {
    fn expanded_dirs(&self) -> &Vec<PathBuf> {
        &self.expanded_dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::state::create_shared_state;
    
    #[tokio::test]
    async fn test_file_explorer_view() {
        let state = create_shared_state().await;
        let view = FileExplorerView::new(state.clone());
        
        assert_eq!(view.view_type(), ViewType::FileExplorer);
        assert_eq!(view.selected_index, 0);
        assert!(view.expanded_dirs().is_empty());
    }
}