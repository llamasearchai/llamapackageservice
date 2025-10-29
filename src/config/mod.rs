mod env_manager;

use std::path::{Path, PathBuf};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::error::{ProcessorError, Result};
use std::fs;
use toml;
use regex;
use std::io;
use tokio;

pub use env_manager::ApiKeys;

/// Main configuration struct for the application
///
/// This structure holds all configuration settings including API tokens,
/// output directories, processing limits, and other settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// GitHub API token for authenticated requests
    pub github_token: Option<String>,
    /// Main output directory for all generated files
    pub output_dir: PathBuf,
    /// Processing configuration for parallel operations
    pub processing: ProcessingConfig,
    /// Rate limit settings for various APIs
    pub rate_limits: RateLimits,
    /// Output file and directory configuration
    pub output_config: OutputConfig,
    /// API keys for various services
    pub api_keys: ApiKeys,
    /// Patterns for files to exclude from processing
    pub excluded_files: Vec<String>,
}

/// Configuration for parallel processing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Maximum number of concurrent downloads
    pub max_concurrent_downloads: usize,
    /// Maximum number of concurrent archive extractions
    pub max_concurrent_extractions: usize,
    /// Maximum number of concurrent code analyses
    pub max_concurrent_analyses: usize,
}

/// Rate limit settings for various APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// GitHub API rate limit (requests per hour)
    pub github_api: u32,
    /// PyPI API rate limit (requests per minute)
    pub pypi_api: u32,
    /// NPM API rate limit (requests per minute)
    pub npm_api: u32,
}

/// Configuration for output files and directories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Base directory for all output
    pub base_dir: PathBuf,
    /// Temporary directory for processing files
    pub temp_dir: PathBuf,
    /// Duration to cache results before refreshing
    pub cache_duration: Duration,
}

impl Config {
    /// Creates a new configuration with the specified output directory
    ///
    /// # Arguments
    /// * `output_dir` - The directory where all output files will be stored
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            output_config: OutputConfig::default(),
            api_keys: ApiKeys::default(),
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            processing: ProcessingConfig::default(),
            rate_limits: RateLimits::default(),
            excluded_files: vec![
                r"\.git/".to_string(),
                r"node_modules/".to_string(),
                r"\.env".to_string(),
            ],
        }
    }

    /// Loads configuration from the default config file location
    ///
    /// If the config file doesn't exist, returns the default configuration.
    /// The config file is expected to be in TOML format.
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ProcessorError::Config("Could not find config directory".into()))?;
        let config_path = config_dir.join("llama-package-service").join("config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| ProcessorError::Message(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| ProcessorError::Message(format!("Failed to parse config file: {}", e)))
    }

    /// Validates the configuration by ensuring necessary directories exist and API tokens are valid
    ///
    /// This method performs a series of validation checks to ensure the configuration
    /// is ready for use in the application.
    pub async fn validate(&self) -> Result<()> {
        self.ensure_directories_exist().await?;
        self.ensure_tokens()?;
        Ok(())
    }

    /// Ensures all output directories required by the application exist
    ///
    /// Creates any missing directories to prepare for file output operations.
    pub async fn ensure_directories_exist(&self) -> Result<()> {
        if !tokio::fs::try_exists(&self.output_dir).await
            .map_err(|e| ProcessorError::IO(e))? {
            tokio::fs::create_dir_all(&self.output_dir).await
                .map_err(|e| ProcessorError::IO(e))?;
        }
        Ok(())
    }

    /// Checks if a file should be excluded from processing based on configured patterns
    ///
    /// # Arguments
    /// * `path` - The file path to check against the exclusion patterns
    ///
    /// # Returns
    /// `true` if the file should be excluded, `false` otherwise
    pub fn is_excluded_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.excluded_files.iter().any(|pattern| {
            if let Ok(regex) = regex::Regex::new(pattern) {
                regex.is_match(&path_str)
            } else {
                false
            }
        })
    }

    /// Validates that all required API tokens are present and valid
    ///
    /// # Returns
    /// `Ok(())` if all required tokens are valid, an error otherwise
    pub fn ensure_tokens(&self) -> Result<()> {
        if let Some(token) = &self.github_token {
            if token.trim().is_empty() {
                return Err(ProcessorError::new("GitHub token is empty"));
            }
        }
        Ok(())
    }

    /// Retrieves the GitHub token from the configuration
    ///
    /// Returns the token as a string if it exists, otherwise returns an error
    pub fn github_token(&self) -> Result<&str> {
        self.github_token.as_deref()
            .ok_or_else(|| ProcessorError::new("GitHub token not configured"))
    }
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 5,
            max_concurrent_extractions: 3,
            max_concurrent_analyses: 2,
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            github_api: 5000,
            pypi_api: 100,
            npm_api: 100,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            temp_dir: std::env::temp_dir(),
            cache_duration: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(PathBuf::from("output"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_validation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = Config::new(temp_dir.path().to_path_buf());
        
        assert!(config.validate().await.is_ok());
        Ok(())
    }

    #[test]
    fn test_github_token() -> Result<()> {
        let config = Config::default();
        assert!(config.github_token().is_err());
        
        let config = Config {
            github_token: Some("test_token".to_string()),
            output_dir: PathBuf::from("output"),
            processing: ProcessingConfig::default(),
            rate_limits: RateLimits::default(),
            output_config: OutputConfig::default(),
            api_keys: ApiKeys::default(),
            excluded_files: vec![],
        };
        assert_eq!(config.github_token()?, "test_token");
        
        Ok(())
    }
} 