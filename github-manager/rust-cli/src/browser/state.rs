use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::error::Result;
use crate::github::{Repository, Issue, PullRequest, Workflow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewType {
    RepositoryList,
    RepositoryDetail,
    FileExplorer,
    Code,
    Search,
    Analytics,
    Workflow,
    Security,
    Help,
}

impl std::fmt::Display for ViewType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewType::RepositoryList => write!(f, "Repository List"),
            ViewType::RepositoryDetail => write!(f, "Repository Detail"),
            ViewType::FileExplorer => write!(f, "File Explorer"),
            ViewType::Code => write!(f, "Code View"),
            ViewType::Search => write!(f, "Search"),
            ViewType::Analytics => write!(f, "Analytics"),
            ViewType::Workflow => write!(f, "Workflows"),
            ViewType::Security => write!(f, "Security"),
            ViewType::Help => write!(f, "Help"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
            Mode::Visual => write!(f, "VISUAL"),
            Mode::Command => write!(f, "COMMAND"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub content: String,
    pub repository: String,
}

#[derive(Debug, Clone)]
pub struct AnalyticsData {
    pub repository: String,
    pub code_quality_score: f64,
    pub test_coverage: f64,
    pub security_score: f64,
    pub open_issues: usize,
    pub open_prs: usize,
    pub last_commit: DateTime<Utc>,
    pub contributors: usize,
    pub languages: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct SecurityIssue {
    pub severity: String,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct BrowserState {
    pub current_view: ViewType,
    pub mode: Mode,
    pub loading: bool,
    pub error: Option<String>,
    
    // Repository data
    pub repositories: Vec<Repository>,
    pub selected_repository: Option<Repository>,
    pub repository_files: Option<FileNode>,
    
    // Navigation
    pub navigation_stack: VecDeque<ViewType>,
    pub list_state: ListState,
    pub scroll_offset: usize,
    
    // Search
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_in_progress: bool,
    
    // Analytics
    pub analytics_data: HashMap<String, AnalyticsData>,
    
    // Security
    pub security_issues: Vec<SecurityIssue>,
    
    // Workflows
    pub workflows: Vec<Workflow>,
    pub workflow_runs: HashMap<String, Vec<WorkflowRun>>,
    
    // File viewing
    pub current_file_path: Option<PathBuf>,
    pub current_file_content: Option<String>,
    pub syntax_highlighted: bool,
    
    // Command mode
    pub command_buffer: String,
    pub command_history: VecDeque<String>,
    pub command_history_index: Option<usize>,
    
    // Status messages
    pub status_message: Option<String>,
    pub status_message_type: StatusMessageType,
}

#[derive(Debug, Clone, Copy)]
pub enum StatusMessageType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Default)]
pub struct ListState {
    pub selected: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub status: String,
    pub conclusion: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            current_view: ViewType::RepositoryList,
            mode: Mode::Normal,
            loading: false,
            error: None,
            repositories: Vec::new(),
            selected_repository: None,
            repository_files: None,
            navigation_stack: VecDeque::with_capacity(10),
            list_state: ListState::default(),
            scroll_offset: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_in_progress: false,
            analytics_data: HashMap::new(),
            security_issues: Vec::new(),
            workflows: Vec::new(),
            workflow_runs: HashMap::new(),
            current_file_path: None,
            current_file_content: None,
            syntax_highlighted: false,
            command_buffer: String::new(),
            command_history: VecDeque::with_capacity(100),
            command_history_index: None,
            status_message: None,
            status_message_type: StatusMessageType::Info,
        }
    }
    
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
    
    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
        if error.is_some() {
            self.status_message_type = StatusMessageType::Error;
            self.status_message = error.clone();
        }
    }
    
    pub fn set_status(&mut self, message: String, message_type: StatusMessageType) {
        self.status_message = Some(message);
        self.status_message_type = message_type;
    }
    
    pub fn navigate_to(&mut self, view: ViewType) {
        if self.current_view != view {
            self.navigation_stack.push_back(self.current_view.clone());
            if self.navigation_stack.len() > 10 {
                self.navigation_stack.pop_front();
            }
            self.current_view = view;
        }
    }
    
    pub fn navigate_back(&mut self) -> bool {
        if let Some(previous) = self.navigation_stack.pop_back() {
            self.current_view = previous;
            true
        } else {
            false
        }
    }
    
    pub fn select_repository(&mut self, index: usize) {
        if index < self.repositories.len() {
            self.selected_repository = Some(self.repositories[index].clone());
            self.list_state.selected = index;
        }
    }
    
    pub fn move_selection_up(&mut self) {
        if self.list_state.selected > 0 {
            self.list_state.selected -= 1;
        }
    }
    
    pub fn move_selection_down(&mut self, max: usize) {
        if self.list_state.selected < max.saturating_sub(1) {
            self.list_state.selected += 1;
        }
    }
    
    pub fn add_command_to_history(&mut self, command: String) {
        if !command.is_empty() {
            self.command_history.push_back(command);
            if self.command_history.len() > 100 {
                self.command_history.pop_front();
            }
        }
        self.command_history_index = None;
    }
    
    pub fn get_previous_command(&mut self) -> Option<String> {
        let index = match self.command_history_index {
            Some(i) if i > 0 => i - 1,
            None if !self.command_history.is_empty() => self.command_history.len() - 1,
            _ => return None,
        };
        
        self.command_history_index = Some(index);
        self.command_history.get(index).cloned()
    }
    
    pub fn get_next_command(&mut self) -> Option<String> {
        let index = match self.command_history_index {
            Some(i) if i < self.command_history.len() - 1 => i + 1,
            _ => return None,
        };
        
        self.command_history_index = Some(index);
        self.command_history.get(index).cloned()
    }
    
    pub async fn refresh(&mut self) -> Result<()> {
        self.set_loading(true);
        self.set_error(None);
        
        // Refresh logic would go here based on current view
        match self.current_view {
            ViewType::RepositoryList => {
                // Refresh repository list
            }
            ViewType::Analytics => {
                // Refresh analytics data
            }
            ViewType::Security => {
                // Refresh security scan
            }
            _ => {}
        }
        
        self.set_loading(false);
        Ok(())
    }
}

pub type SharedState = Arc<RwLock<BrowserState>>;

pub async fn create_shared_state() -> SharedState {
    Arc::new(RwLock::new(BrowserState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_state_navigation() {
        let mut state = BrowserState::new();
        assert_eq!(state.current_view, ViewType::RepositoryList);
        
        state.navigate_to(ViewType::RepositoryDetail);
        assert_eq!(state.current_view, ViewType::RepositoryDetail);
        assert_eq!(state.navigation_stack.len(), 1);
        
        state.navigate_to(ViewType::FileExplorer);
        assert_eq!(state.current_view, ViewType::FileExplorer);
        assert_eq!(state.navigation_stack.len(), 2);
        
        assert!(state.navigate_back());
        assert_eq!(state.current_view, ViewType::RepositoryDetail);
        assert_eq!(state.navigation_stack.len(), 1);
    }
    
    #[test]
    fn test_list_state_movement() {
        let mut state = BrowserState::new();
        state.repositories = vec![
            Repository::default(),
            Repository::default(),
            Repository::default(),
        ];
        
        assert_eq!(state.list_state.selected, 0);
        
        state.move_selection_down(3);
        assert_eq!(state.list_state.selected, 1);
        
        state.move_selection_down(3);
        assert_eq!(state.list_state.selected, 2);
        
        state.move_selection_down(3);
        assert_eq!(state.list_state.selected, 2); // Should not go beyond
        
        state.move_selection_up();
        assert_eq!(state.list_state.selected, 1);
    }
    
    #[test]
    fn test_command_history() {
        let mut state = BrowserState::new();
        
        state.add_command_to_history("search TODO".to_string());
        state.add_command_to_history("analyze repo".to_string());
        
        assert_eq!(state.get_previous_command(), Some("analyze repo".to_string()));
        assert_eq!(state.get_previous_command(), Some("search TODO".to_string()));
        assert_eq!(state.get_previous_command(), None);
        
        assert_eq!(state.get_next_command(), Some("analyze repo".to_string()));
    }
}