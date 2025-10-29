use std::fmt;
use thiserror::Error;
use std::io;
use async_openai::error::OpenAIError;

/// Custom result type alias for the application
pub type Result<T> = std::result::Result<T, ProcessorError>;

/// Errors that can occur during package processing
#[derive(Debug, Error)]
pub enum ProcessorError {
    /// I/O errors
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    
    /// HTTP request/response errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    /// JSON parsing/serialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// ZIP file processing errors
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    
    /// Directory traversal errors
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
    
    /// URL parsing errors
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    /// File download errors
    #[error("Download error: {0}")]
    Download(String),
    
    /// General message errors
    #[error("{0}")]
    Message(String),
    
    /// Network connectivity errors
    #[error("Network error: {0}")]
    Network(String),
    
    /// Input validation errors
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Package processing errors
    #[error("Processing error: {0}")]
    Processing(String),
    
    /// Configuration errors
    #[error("Config error: {0}")]
    Config(String),
    
    /// API rate limit exceeded errors
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    /// GitHub API specific errors
    #[error("GitHub API error: {0}")]
    GitHubApi(String),
    
    /// Language Model errors
    #[error("LLM error: {0}")]
    LLM(String),
    
    /// Parsing errors
    #[error("Parsing error: {0}")]
    Parse(String),
    
    /// Database errors
    #[error("Database error: {0}")]
    Database(String),
    
    /// OpenAI API errors
    #[error("OpenAI error: {0}")]
    OpenAI(#[from] OpenAIError),
    
    /// PyPI API errors
    #[error("PyPI API error: {0}")]
    PyPiApi(String),
    
    /// NPM API errors
    #[error("NPM API error: {0}")]
    NpmApi(String),
    
    /// Cache errors
    #[error("Cache error: {0}")]
    Cache(String),
}

impl ProcessorError {
    /// Creates a new error with the specified message
    pub fn new(message: &str) -> Self {
        Self::Message(message.to_string())
    }
    
    /// Checks if this error is transient and retryable
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | 
            Self::Http(_) | 
            Self::RateLimitExceeded(_) |
            Self::IO(_)
        )
    }
    
    /// Checks if this error is fatal and should terminate processing
    pub fn is_fatal(&self) -> bool {
        !self.is_transient()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let error = ProcessorError::new("test error");
        assert!(matches!(error, ProcessorError::Message(_)));
        
        if let ProcessorError::Message(msg) = error {
            assert_eq!(msg, "test error");
        }
    }
    
    #[test]
    fn test_is_transient() {
        let transient = ProcessorError::Network("connection timeout".into());
        let fatal = ProcessorError::Validation("invalid input".into());
        
        assert!(transient.is_transient());
        assert!(!fatal.is_transient());
    }
}
