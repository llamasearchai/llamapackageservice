#!/bin/bash
set -e

echo "🔧 Fixing configuration..."

cat > src/config.rs << 'EOL'
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{ProcessorError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub github_token: Option<String>,
    pub output_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub max_concurrent_downloads: usize,
    pub telemetry: TelemetryConfig,
    pub mlx_settings: MLXSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLXSettings {
    pub enabled: bool,
    pub endpoint: String,
    pub timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: None,
            output_dir: PathBuf::from("output"),
            cache_dir: PathBuf::from(".cache"),
            max_concurrent_downloads: 4,
            telemetry: TelemetryConfig {
                enabled: false,
                endpoint: "https://telemetry.example.com".to_string(),
            },
            mlx_settings: MLXSettings {
                enabled: false,
                endpoint: "http://localhost:5001/v1".to_string(),
                timeout_seconds: 30,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        
        if !config_path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| ProcessorError::Config(format!("Failed to read config: {}", e)))?;
            
        serde_json::from_str(&content)
            .map_err(|e| ProcessorError::Config(format!("Failed to parse config: {}", e)))
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ProcessorError::Config(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProcessorError::Config(format!("Failed to serialize config: {}", e)))?;
            
        fs::write(&config_path, content)
            .map_err(|e| ProcessorError::Config(format!("Failed to write config: {}", e)))
    }
    
    fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".code2txt").join("config.json")
    }
}
EOL

echo "✅ Configuration fixed!"
