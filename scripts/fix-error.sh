#!/bin/bash
set -e

echo "🔧 Fixing error handling..."

cat > src/error.rs << 'EOL'
use std::fmt;
use thiserror::Error;

// Fix the Result type alias
pub type Result<T> = std::result::Result<T, ProcessorError>;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
    
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("Download error: {0}")]
    Download(String),
    
    #[error("{0}")]
    Message(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("GitHub API error: {0}")]
    GitHubApi(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("LLM error: {0}")]
    LLM(String),
}

impl ProcessorError {
    pub fn new(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}
EOL

echo "✅ Error handling fixed!"
