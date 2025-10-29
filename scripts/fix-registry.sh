#!/bin/bash
set -e

echo "🔧 Fixing processor registry..."

cat > src/processors/registry.rs << 'EOL'
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::Config;
use crate::error::Result;
use crate::processors::{PackageProcessor, github::GitHubProcessor};

pub struct ProcessorRegistry {
    processors: Arc<RwLock<HashMap<&'static str, Box<dyn PackageProcessor + Send + Sync>>>>,
}

impl ProcessorRegistry {
    pub fn new() -> Self {
        let mut processors = HashMap::new();
        
        // Register built-in processors
        processors.insert("github", Box::new(GitHubProcessor::new()) as Box<dyn PackageProcessor + Send + Sync>);
        
        Self {
            processors: Arc::new(RwLock::new(processors)),
        }
    }
    
    pub async fn register<P>(&self, name: &'static str, processor: P)
    where
        P: PackageProcessor + Send + Sync + 'static,
    {
        let mut processors = self.processors.write().await;
        processors.insert(name, Box::new(processor));
    }
    
    pub async fn get(&self, name: &str) -> Option<Box<dyn PackageProcessor + Send + Sync + 'static>> {
        let processors = self.processors.read().await;
        
        match name {
            "github" => Some(Box::new(GitHubProcessor::new())),
            _ => None,
        }
    }
    
    pub async fn find_processor_for_url(&self, url: &str) -> Option<Box<dyn PackageProcessor + Send + Sync + 'static>> {
        let processors = self.processors.read().await;
        
        for (name, processor) in processors.iter() {
            if processor.accepts(url) {
                // Create a fresh instance to avoid lifetime issues
                match *name {
                    "github" => return Some(Box::new(GitHubProcessor::new())),
                    _ => continue,
                }
            }
        }
        
        None
    }
    
    pub async fn process_url(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()> {
        let processor = self.find_processor_for_url(url).await
            .ok_or_else(|| crate::error::ProcessorError::NoSuitableProcessor(url.to_string()))?;
            
        processor.process(url, output_dir, config).await
    }
}
EOL

echo "✅ Processor registry fixed!"
