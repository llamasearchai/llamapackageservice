pub mod repository_list;
pub mod repository_detail;
pub mod file_explorer;
pub mod code_view;
pub mod search_view;
pub mod analytics_view;
pub mod workflow_view;
pub mod security_view;
pub mod help_view;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::{backend::Backend, layout::Rect, Frame};

use crate::error::Result;
use crate::browser::state::{SharedState, ViewType};

pub use self::{
    repository_list::RepositoryListView,
    repository_detail::RepositoryDetailView,
    file_explorer::FileExplorerView,
    code_view::CodeView,
    search_view::SearchView,
    analytics_view::AnalyticsView,
    workflow_view::WorkflowView,
    security_view::SecurityView,
    help_view::HelpView,
};

#[async_trait]
pub trait View: Send + Sync {
    fn view_type(&self) -> ViewType;
    
    async fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) -> Result<()>;
    
    async fn handle_input(&mut self, key: KeyEvent) -> Result<bool>;
    
    async fn on_enter(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn on_exit(&mut self) -> Result<()> {
        Ok(())
    }
}