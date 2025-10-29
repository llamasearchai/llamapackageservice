use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use dirs;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub github: GitHubConfig,
    
    #[serde(default)]
    pub paths: PathsConfig,
    
    #[serde(default)]
    pub ui: UIConfig,
    
    #[serde(default)]
    pub security: SecurityConfig,
    
    #[serde(default)]
    pub performance: PerformanceConfig,
    
    #[serde(default)]
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub organization: String,
    pub token: Option<String>,
    pub api_url: String,
    pub per_page: u32,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub repos_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub export_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub database_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub refresh_rate_ms: u64,
    pub show_hidden_files: bool,
    pub syntax_theme: String,
    pub max_file_size_mb: u64,
    pub default_view: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub scan_on_clone: bool,
    pub auto_fix: bool,
    pub severity_threshold: String,
    pub ignored_patterns: Vec<String>,
    pub secret_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_parallel_operations: usize,
    pub cache_size_mb: u64,
    pub enable_compression: bool,
    pub chunk_size_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub auto_sync: bool,
    pub real_time_updates: bool,
    pub ai_analysis: bool,
    pub workflow_automation: bool,
    pub export_formats: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github: GitHubConfig::default(),
            paths: PathsConfig::default(),
            ui: UIConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeaturesConfig::default(),
        }
    }
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            organization: "llamasearchai".to_string(),
            token: None,
            api_url: "https://api.github.com".to_string(),
            per_page: 100,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base = home.join(".llamasearch");
        
        Self {
            repos_dir: base.join("repos"),
            cache_dir: base.join("cache"),
            export_dir: base.join("exports"),
            logs_dir: base.join("logs"),
            database_path: base.join("db"),
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            refresh_rate_ms: 250,
            show_hidden_files: false,
            syntax_theme: "base16-ocean.dark".to_string(),
            max_file_size_mb: 10,
            default_view: "repository_list".to_string(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            scan_on_clone: true,
            auto_fix: false,
            severity_threshold: "medium".to_string(),
            ignored_patterns: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
            ],
            secret_patterns: vec![
                r"api[_-]?key\s*=\s*['\"][^'\"]+['\"]".to_string(),
                r"token\s*=\s*['\"][^'\"]+['\"]".to_string(),
                r"password\s*=\s*['\"][^'\"]+['\"]".to_string(),
                r"secret\s*=\s*['\"][^'\"]+['\"]".to_string(),
            ],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_parallel_operations: 4,
            cache_size_mb: 512,
            enable_compression: true,
            chunk_size_kb: 64,
        }
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            real_time_updates: true,
            ai_analysis: true,
            workflow_automation: true,
            export_formats: vec![
                "txt".to_string(),
                "markdown".to_string(),
                "json".to_string(),
                "html".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        // Expand ~ to home directory
        let expanded_path = if path.starts_with("~") {
            dirs::home_dir()
                .ok_or_else(|| Error::Config("Could not determine home directory".to_string()))?
                .join(path.strip_prefix("~").unwrap())
        } else {
            path.to_path_buf()
        };
        
        // If config doesn't exist, create default
        if !expanded_path.exists() {
            let config = Self::default();
            config.save(&expanded_path)?;
            return Ok(config);
        }
        
        // Load from file
        let content = fs::read_to_string(&expanded_path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;
        
        // Try to parse as TOML first
        if expanded_path.extension().and_then(|e| e.to_str()) == Some("toml") {
            toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse TOML config: {}", e)))
        } else {
            // Try YAML
            serde_yaml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse YAML config: {}", e)))
        }
    }
    
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
        }
        
        // Serialize based on extension
        let content = if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            toml::to_string_pretty(self)
                .map_err(|e| Error::Config(format!("Failed to serialize config to TOML: {}", e)))?
        } else {
            serde_yaml::to_string(self)
                .map_err(|e| Error::Config(format!("Failed to serialize config to YAML: {}", e)))?
        };
        
        fs::write(path, content)
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate GitHub config
        if self.github.organization.is_empty() {
            return Err(Error::Validation("GitHub organization cannot be empty".to_string()));
        }
        
        // Validate paths exist or can be created
        for path in [
            &self.paths.repos_dir,
            &self.paths.cache_dir,
            &self.paths.export_dir,
            &self.paths.logs_dir,
            self.paths.database_path.parent().unwrap_or(Path::new(".")),
        ] {
            if !path.exists() {
                fs::create_dir_all(path)
                    .map_err(|e| Error::Config(format!("Failed to create directory {:?}: {}", path, e)))?;
            }
        }
        
        // Validate performance settings
        if self.performance.max_parallel_operations == 0 {
            return Err(Error::Validation("max_parallel_operations must be at least 1".to_string()));
        }
        
        if self.performance.cache_size_mb == 0 {
            return Err(Error::Validation("cache_size_mb must be greater than 0".to_string()));
        }
        
        Ok(())
    }
    
    pub fn merge_env_vars(&mut self) {
        // Override with environment variables if present
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            self.github.token = Some(token);
        }
        
        if let Ok(org) = std::env::var("LLAMASEARCH_ORG") {
            self.github.organization = org;
        }
        
        if let Ok(repos_dir) = std::env::var("LLAMASEARCH_REPOS_DIR") {
            self.paths.repos_dir = PathBuf::from(repos_dir);
        }
    }
    
    pub fn ensure_directories(&self) -> Result<()> {
        for path in [
            &self.paths.repos_dir,
            &self.paths.cache_dir,
            &self.paths.export_dir,
            &self.paths.logs_dir,
            self.paths.database_path.parent().unwrap_or(Path::new(".")),
        ] {
            if !path.exists() {
                fs::create_dir_all(path)
                    .map_err(|e| Error::Config(format!("Failed to create directory {:?}: {}", path, e)))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.github.organization, "llamasearchai");
        assert_eq!(config.ui.theme, "dark");
        assert!(config.features.auto_sync);
    }
    
    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut config = Config::default();
        config.github.organization = "test_org".to_string();
        config.ui.theme = "light".to_string();
        
        config.save(&config_path).unwrap();
        
        let loaded_config = Config::load(&config_path).unwrap();
        assert_eq!(loaded_config.github.organization, "test_org");
        assert_eq!(loaded_config.ui.theme, "light");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        config.github.organization = String::new();
        assert!(config.validate().is_err());
        
        config.github.organization = "llamasearchai".to_string();
        config.performance.max_parallel_operations = 0;
        assert!(config.validate().is_err());
    }
}