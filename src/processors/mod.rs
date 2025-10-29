use async_trait::async_trait;
use crate::error::{ProcessorError, Result};
use crate::config::Config;
use std::path::Path;

/// Module for GitHub repository processing
pub mod github;
/// Module for common processor utilities
pub mod common;
/// Module for PyPI package processing
pub mod pypi;
/// Module for processor registration
pub mod registry;
/// Module for Go package processing
pub mod go;
/// Module for PyPI profile processing
pub mod pypi_profile;
/// Module for NPM package processing
pub mod npm;
/// Module for Rust crate processing
pub mod crates;
/// Module for local file/directory processing  
pub mod local;

/// Interface for package processors
#[async_trait]
pub trait PackageProcessor: Send + Sync {
    /// Processes a package from the given URL and writes output to the specified directory
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()>;
    /// Returns the name of the processor
    fn name(&self) -> &'static str;
    /// Determines if this processor can handle the given URL
    fn accepts(&self, url: &str) -> bool;
    /// Validates that the URL can be processed before starting
    async fn validate(&self, url: &str) -> Result<()>;
}

/// Factory for creating package processors
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Creates the appropriate processor for the given URL
    pub fn create_processor(url: &str) -> Result<Box<dyn PackageProcessor + Send + Sync>> {
        let url_lower = url.to_lowercase();
        
        // Local file/directory detection (check first, before URL-based processors)
        if local::LocalProcessor::is_local_path(url) {
            return Ok(Box::new(local::LocalProcessor::new()));
        }
        
        // Go package detection
        if url_lower.contains("pkg.go.dev") || url_lower.contains("golang.org/pkg") {
            return Ok(Box::new(go::GoProcessor::new()?));
        }
        
        // GitHub repository detection
        if url_lower.contains("github.com") {
            return Ok(Box::new(github::GitHubProcessor::new()));
        }
        
        // PyPI profile detection
        if url_lower.contains("pypi.org/user/") {
            return Ok(Box::new(PyPiProfileProcessor::new()));
        }
        
        // PyPI package detection
        if url_lower.contains("pypi.org/project/") || 
           url_lower.contains("pip install") || 
           url.starts_with("pip ") {
            return Ok(Box::new(pypi::PyPiProcessor::new()));
        }
        
        // NPM package detection
        if url_lower.contains("npmjs.com") || 
           url_lower.contains("npm install") || 
           url.starts_with("npm ") {
            return Ok(Box::new(npm::NpmProcessor::new()));
        }
        
        // Rust crate detection
        if url_lower.contains("crates.io") || 
           url_lower.contains("docs.rs") || 
           url.starts_with("cargo add ") {
            return Ok(Box::new(crates::CratesProcessor::new()));
        }
        
        // Try to guess the type from domain or format
        if url_lower.contains("gitlab.com") || url_lower.contains("bitbucket.org") {
            // Treat other repository hosts like GitHub
            return Ok(Box::new(github::GitHubProcessor::new()));
        }
        
        Err(ProcessorError::Message(format!(
            "Unsupported URL: {}. Please use a URL from a supported source (GitHub, PyPI, NPM, crates.io, pkg.go.dev) or provide a local file/directory path",
            url
        )))
    }
    
    /// Detects the type of URL (for information only)
    pub fn detect_url_type(url: &str) -> String {
        let url_lower = url.to_lowercase();
        
        if local::LocalProcessor::is_local_path(url) {
            return "Local File/Directory".to_string();
        } else if url_lower.contains("github.com") {
            return "GitHub Repository".to_string();
        } else if url_lower.contains("pypi.org/user/") {
            return "PyPI User Profile".to_string();
        } else if url_lower.contains("pypi.org") || url_lower.contains("pip install") {
            return "PyPI Package".to_string();
        } else if url_lower.contains("npmjs.com") || url_lower.contains("npm install") {
            return "NPM Package".to_string();
        } else if url_lower.contains("crates.io") || url_lower.contains("docs.rs") {
            return "Rust Crate".to_string();
        } else if url_lower.contains("pkg.go.dev") || url_lower.contains("golang.org/pkg") {
            return "Go Package".to_string();
        } else {
            return "Unknown".to_string();
        }
    }
}

/// Processor for PyPI user profiles
///
/// This processor handles the analysis and documentation of 
/// PyPI user profiles, collecting information about a user's
/// packages and contributions.
pub struct PyPiProfileProcessor {
    /// HTTP client for making requests to the PyPI API
    client: reqwest::Client,
}

impl PyPiProfileProcessor {
    /// Creates a new PyPI profile processor with default configuration
    ///
    /// # Returns
    /// A new PyPiProfileProcessor instance
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
    
    fn extract_username(&self, url: &str) -> Result<String> {
        // Extract username from URL like https://pypi.org/user/username/
        if let Some(username) = url.split("/user/").nth(1) {
            let username = username.trim_end_matches('/');
            if !username.is_empty() {
                return Ok(username.to_string());
            }
        }
        Err(ProcessorError::Message(format!("Invalid PyPI profile URL: {}", url)))
    }
}

#[async_trait]
impl PackageProcessor for PyPiProfileProcessor {
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let username = self.extract_username(url)?;
        let pb = indicatif::ProgressBar::new_spinner();
        common::setup_progress_style(&pb);
        pb.set_message(format!("Processing PyPI profile: {}", username));
        
        // Just call the function directly
        pypi_profile::process_pypi_profile(&username, output_dir, &pb).await
    }
    
    fn name(&self) -> &'static str {
        "PyPI Profile Processor"
    }
    
    fn accepts(&self, url: &str) -> bool {
        url.contains("pypi.org/user/")
    }
    
    async fn validate(&self, url: &str) -> Result<()> {
        self.extract_username(url)?;
        Ok(())
    }
}

impl Clone for PyPiProfileProcessor {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_processor_creation() -> Result<()> {
        let github_url = "https://github.com/owner/repo";
        let npm_url = "https://www.npmjs.com/package/test";
        let pypi_url = "https://pypi.org/project/test";
        let pypi_profile_url = "https://pypi.org/user/username";
        let crates_url = "https://crates.io/crates/test";
        let invalid_url = "https://invalid.com";

        assert!(create_processor(github_url).is_some());
        assert!(create_processor(npm_url).is_some());
        assert!(create_processor(pypi_url).is_some());
        assert!(create_processor(pypi_profile_url).is_some());
        assert!(create_processor(crates_url).is_some());
        assert!(create_processor(invalid_url).is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_processor_validation() -> Result<()> {
        let github_url = "https://github.com/owner/repo";
        let processor = create_processor(github_url).unwrap();
        let result = processor.validate(github_url).await;
        assert!(result.is_ok());
        Ok(())
    }
}

// Re-export processor functions
pub use self::{
    github::process_github_url,
    pypi::process_pypi_url as process_pypi_package,
    npm::process_npm_package,
    crates::process_crate,
    go::process_go_package,
};

// Re-export common types
pub use self::{
    pypi::PyPiProcessor,
    crates::CrateInfo,
};

pub use pypi_profile::process_pypi_profile;

/// Creates the appropriate processor for the given URL
pub fn create_processor(url: &str) -> Option<Box<dyn PackageProcessor + Send + Sync>> {
    ProcessorFactory::create_processor(url).ok()
}