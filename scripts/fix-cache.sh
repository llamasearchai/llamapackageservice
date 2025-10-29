#!/bin/bash
set -e

echo "🔧 Fixing cache implementation..."

cat > src/cache.rs << 'EOL'
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub source: String,
    pub version: String,
    pub ttl: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub ttl: u64,
}

pub struct Cache {
    base_dir: PathBuf,
}

impl Cache {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Cache {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    fn get_path(&self, key: &str) -> PathBuf {
        let hash = format!("{:x}", md5::compute(key));
        let dir = &hash[0..2];
        self.base_dir.join(dir).join(&hash)
    }

    fn get_metadata_path(&self, key: &str) -> PathBuf {
        let mut path = self.get_path(key);
        path.set_extension("meta");
        path
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let path = self.get_path(key);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        Ok(Some(content))
    }

    pub async fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.get_path(key);
        let meta_path = self.get_metadata_path(key);
        
        if path.exists() {
            fs::remove_file(&path)?;
        }
        
        if meta_path.exists() {
            fs::remove_file(&meta_path)?;
        }
        
        Ok(())
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let path = self.get_path(key);
        
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        fs::write(&path, value)?;
        Ok(())
    }

    pub async fn set_with_metadata(&self, key: &str, value: &str, metadata: &CacheMetadata) -> Result<()> {
        // First save the data
        self.set(key, value).await?;
        
        // Then save metadata
        let meta_path = self.get_metadata_path(key);
        let entry = CacheEntry {
            url: key.to_string(),
            path: self.get_path(key),
            timestamp: Utc::now(),
            ttl: metadata.ttl,
        };
        
        fs::write(&meta_path, serde_json::to_string(&entry)?)?;
        Ok(())
    }
}
EOL

echo "✅ Cache implementation fixed!"
