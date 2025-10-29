use std::fs;
use std::path::PathBuf;
use dialoguer::{Input, Password};
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use crate::error::{ProcessorError, Result};
use std::env;
use std::io;
use dirs;
use dialoguer;

/// Stores API keys for various services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeys {
    /// GitHub API token for authenticated requests
    pub github_token: Option<String>,
    /// PyPI token for authenticated requests
    pub pypi_token: Option<String>,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            github_token: None,
            pypi_token: None,
        }
    }
}

impl ApiKeys {
    /// Loads API keys from the environment or configuration file
    pub fn load() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "llamasearch", "llama-package-service")
            .ok_or_else(|| ProcessorError::Config("Could not determine project directories".into()))?;
        let config_dir = project_dirs.config_dir();
        let config_file = config_dir.join("config.toml");

        if !config_file.exists() {
            fs::create_dir_all(config_dir)
                .map_err(|e| ProcessorError::Config(format!("Failed to create config directory: {}", e)))?;
            
            let github_token: String = Input::new()
                .with_prompt("Enter your GitHub token")
                .interact_text()
                .map_err(|e| ProcessorError::Config(e.to_string()))?;
                
            fs::write(&config_file, format!("github_token = \"{}\"", github_token))
                .map_err(|e| ProcessorError::Config(format!("Failed to write config file: {}", e)))?;
        }

        let config_content = fs::read_to_string(config_file)
            .map_err(|e| ProcessorError::Config(format!("Failed to read config file: {}", e)))?;
            
        let github_token = config_content
            .lines()
            .find(|line| line.starts_with("github_token"))
            .and_then(|line| line.split('=').nth(1))
            .map(|token| token.trim().trim_matches('"').to_string());

        Ok(Self {
            github_token,
            pypi_token: None,
        })
    }

    /// Saves the current API keys to the configuration file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Ensure the directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ProcessorError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProcessorError::Config(format!("Failed to serialize config: {}", e)))?;
            
        fs::write(&config_path, content)
            .map_err(|e| ProcessorError::Config(format!("Failed to write config: {}", e)))
    }

    /// Prompts the user to input any missing API keys
    pub fn prompt_missing_keys(&mut self) -> Result<()> {
        if self.github_token.is_none() {
            self.github_token = dialoguer::Input::new()
                .with_prompt("Enter GitHub token")
                .allow_empty(true)
                .interact_text()
                .map(|s: String| if s.is_empty() { None } else { Some(s) })
                .map_err(|e| ProcessorError::Config(e.to_string()))?;
        }
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "llamasearch", "package-processor")
            .ok_or_else(|| ProcessorError::Config("Failed to determine config directory".into()))?;
            
        Ok(proj_dirs.config_dir().join("api_keys.json"))
    }
}

pub fn get_env_value(key: &str) -> Option<String> {
    let value = std::env::var(key).ok()?;
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
