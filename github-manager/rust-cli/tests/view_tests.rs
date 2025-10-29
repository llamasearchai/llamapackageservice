use llamasearch_cli::browser::state::{create_shared_state, ViewType};
use llamasearch_cli::browser::views::{View, RepositoryListView, HelpView};

#[tokio::test]
async fn test_repository_list_view() {
    let state = create_shared_state().await;
    let view = RepositoryListView::new(state.clone());
    
    assert_eq!(view.view_type(), ViewType::RepositoryList);
}

#[tokio::test]
async fn test_help_view() {
    let view = HelpView::new();
    assert_eq!(view.view_type(), ViewType::Help);
}

#[tokio::test]
async fn test_view_navigation() {
    let state = create_shared_state().await;
    
    {
        let mut s = state.write().await;
        s.navigate_to(ViewType::RepositoryDetail);
        assert_eq!(s.current_view, ViewType::RepositoryDetail);
        
        s.navigate_to(ViewType::FileExplorer);
        assert_eq!(s.current_view, ViewType::FileExplorer);
        
        assert!(s.navigate_back());
        assert_eq!(s.current_view, ViewType::RepositoryDetail);
    }
}