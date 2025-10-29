use clap::Parser;
use color_eyre::Result;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

mod app;
mod browser;
mod cli;
mod commands;
mod config;
mod error;
mod github;
mod security;
mod tui;
mod utils;

use crate::app::App;
use crate::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;
    
    // Initialize logging
    init_logging()?;
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = config::Config::load(&cli.config)?;
    
    // Handle commands
    match cli.command {
        Some(Commands::Tui) => {
            // Launch TUI browser
            info!("Launching terminal UI browser");
            let mut app = App::new(config).await?;
            app.run().await?;
        }
        Some(cmd) => {
            // Execute command
            commands::execute(cmd, config).await?;
        }
        None => {
            // No command specified, show help
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }
    
    Ok(())
}

fn init_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_parsing() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}