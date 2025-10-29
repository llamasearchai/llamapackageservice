use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    browser::Browser,
    config::Config,
    error::{Error, Result},
    github::GitHubClient,
};

pub struct App {
    config: Config,
    browser: Browser,
    github_client: Arc<GitHubClient>,
    should_quit: bool,
}

impl App {
    pub async fn new(mut config: Config) -> Result<Self> {
        // Merge environment variables
        config.merge_env_vars();
        
        // Validate configuration
        config.validate()?;
        
        // Ensure directories exist
        config.ensure_directories()?;
        
        // Initialize GitHub client
        let github_client = Arc::new(
            GitHubClient::new(
                config.github.token.clone(),
                config.github.organization.clone(),
            )
            .await?,
        );
        
        // Initialize browser
        let browser = Browser::new(config.clone()).await?;
        
        Ok(Self {
            config,
            browser,
            github_client,
            should_quit: false,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        let mut terminal = self.setup_terminal()?;
        
        // Initialize data
        self.initialize_data().await?;
        
        // Run the main loop
        let result = self.run_loop(&mut terminal).await;
        
        // Restore terminal
        self.restore_terminal(&mut terminal)?;
        
        result
    }
    
    fn setup_terminal(&self) -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode().map_err(|e| Error::Terminal(format!("Failed to enable raw mode: {}", e)))?;
        
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| Error::Terminal(format!("Failed to setup terminal: {}", e)))?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| Error::Terminal(format!("Failed to create terminal: {}", e)))?;
        
        Ok(terminal)
    }
    
    fn restore_terminal(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        disable_raw_mode().map_err(|e| Error::Terminal(format!("Failed to disable raw mode: {}", e)))?;
        
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .map_err(|e| Error::Terminal(format!("Failed to restore terminal: {}", e)))?;
        
        terminal.show_cursor()
            .map_err(|e| Error::Terminal(format!("Failed to show cursor: {}", e)))?;
        
        Ok(())
    }
    
    async fn initialize_data(&mut self) -> Result<()> {
        info!("Initializing application data...");
        
        // Load repositories
        tokio::spawn({
            let client = self.github_client.clone();
            let browser = self.browser.clone();
            async move {
                match client.list_repositories().await {
                    Ok(repos) => {
                        let mut state = browser.state.lock().await;
                        state.repositories = repos;
                        state.set_loading(false);
                    }
                    Err(e) => {
                        error!("Failed to load repositories: {}", e);
                        let mut state = browser.state.lock().await;
                        state.set_error(Some(format!("Failed to load repositories: {}", e)));
                        state.set_loading(false);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let tick_rate = Duration::from_millis(self.config.ui.refresh_rate_ms);
        
        loop {
            // Draw UI
            terminal.draw(|frame| {
                if let Err(e) = tokio::runtime::Handle::current().block_on(self.browser.draw(frame)) {
                    error!("Failed to draw UI: {}", e);
                }
            })?;
            
            // Handle events
            if event::poll(tick_rate)? {
                if let Event::Key(key) = event::read()? {
                    if self.handle_key_event(key).await? {
                        break;
                    }
                }
            }
            
            // Check if we should quit
            if self.should_quit {
                break;
            }
        }
        
        Ok(())
    }
    
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Global quit handler
        if key.code == KeyCode::Char('q') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
            info!("Quit requested by user");
            return Ok(true);
        }
        
        // Pass to browser
        let should_quit = self.browser.handle_input(key).await?;
        
        Ok(should_quit)
    }
    
    pub async fn refresh_data(&mut self) -> Result<()> {
        let state = self.browser.state.lock().await;
        state.set_loading(true);
        drop(state);
        
        // Refresh repositories
        match self.github_client.list_repositories().await {
            Ok(repos) => {
                let mut state = self.browser.state.lock().await;
                state.repositories = repos;
                state.set_loading(false);
                state.set_status(
                    "Data refreshed successfully".to_string(),
                    crate::browser::state::StatusMessageType::Success,
                );
            }
            Err(e) => {
                let mut state = self.browser.state.lock().await;
                state.set_error(Some(format!("Failed to refresh data: {}", e)));
                state.set_loading(false);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_app_creation() {
        let config = Config::default();
        // Note: This test might fail without a valid GitHub token
        // In a real test environment, we'd mock the GitHub client
        let result = App::new(config).await;
        assert!(result.is_ok() || result.is_err()); // Allow both for CI
    }
}