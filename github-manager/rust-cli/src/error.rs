use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("GitHub API error: {0}")]
    GitHub(#[from] octocrab::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Repository not found: {0}")]
    RepoNotFound(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Security error: {0}")]
    Security(String),
    
    #[error("Workflow error: {0}")]
    Workflow(String),
    
    #[error("Terminal error: {0}")]
    Terminal(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sled::Error),
    
    #[error("Encoding error: {0}")]
    Encoding(#[from] bincode::Error),
    
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    
    #[error("Syntax highlighting error: {0}")]
    SyntaxHighlight(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(_) | Error::Timeout | Error::RateLimit | Error::Http(_)
        )
    }
    
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Error::Auth(_))
    }
    
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::RepoNotFound(_) | Error::FileNotFound(_))
    }
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: String,
    pub context: Option<String>,
    pub suggestion: Option<String>,
}

impl ErrorContext {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            context: None,
            suggestion: None,
        }
    }
    
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
    
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

pub trait ErrorExt<T> {
    fn context(self, msg: &str) -> Result<T>;
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T> ErrorExt<T> for Result<T> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| Error::Unknown(format!("{}: {}", msg, e)))
    }
    
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| Error::Unknown(format!("{}: {}", f(), e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_types() {
        let io_err = Error::Io(io::Error::new(io::ErrorKind::NotFound, "test"));
        assert!(matches!(io_err, Error::Io(_)));
        
        let auth_err = Error::Auth("Invalid token".to_string());
        assert!(auth_err.is_auth_error());
        
        let network_err = Error::Network("Connection failed".to_string());
        assert!(network_err.is_retryable());
    }
    
    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new("Test error")
            .with_context("During file operation")
            .with_suggestion("Check file permissions");
        
        assert_eq!(ctx.error, "Test error");
        assert_eq!(ctx.context.unwrap(), "During file operation");
        assert_eq!(ctx.suggestion.unwrap(), "Check file permissions");
    }
}