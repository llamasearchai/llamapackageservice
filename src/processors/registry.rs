use crate::error::Result;
use crate::config::Config;
use crate::processors::PackageProcessor;
use crate::processors::{github, pypi, npm, crates};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing and accessing package processors
///
/// This registry stores and provides access to various package processors
/// (GitHub, PyPI, NPM, Crates), allowing the application to retrieve the 
/// appropriate processor for each type of package.
pub struct ProcessorRegistry {
    /// Concurrent map of processor names to processor instances
    processors: Arc<RwLock<HashMap<String, Box<dyn PackageProcessor + Send + Sync + 'static>>>>,
}

impl ProcessorRegistry {
    /// Creates a new empty processor registry
    ///
    /// # Returns
    /// A new ProcessorRegistry with no registered processors
    pub fn new() -> Self {
        Self {
            processors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a package processor with the registry
    ///
    /// # Arguments
    /// * `name` - The name to register the processor under
    /// * `processor` - The processor instance to register
    pub async fn register<P>(&self, name: &str, processor: P)
    where
        P: PackageProcessor + Send + Sync + 'static,
    {
        let mut processors = self.processors.write().await;
        processors.insert(name.to_string(), Box::new(processor));
    }

    /// Retrieves a processor by name, creating a new instance if found
    ///
    /// # Arguments
    /// * `name` - The name of the processor to retrieve
    ///
    /// # Returns
    /// Some(processor) if the name is registered, None otherwise
    pub async fn get(&self, name: &str) -> Option<Box<dyn PackageProcessor + Send + Sync + 'static>> {
        let processors = self.processors.read().await;
        processors.get(name).map(|_p| {
            match name {
                "GitHub" => Box::new(github::GitHubProcessor::new()) as Box<dyn PackageProcessor + Send + Sync + 'static>,
                "PyPI" => Box::new(pypi::PyPiProcessor::new()) as Box<dyn PackageProcessor + Send + Sync + 'static>,
                "npm" => Box::new(npm::NpmProcessor::new()) as Box<dyn PackageProcessor + Send + Sync + 'static>,
                "crates" => Box::new(crates::CratesProcessor::new()) as Box<dyn PackageProcessor + Send + Sync + 'static>,
                _ => Box::new(github::GitHubProcessor::new()) as Box<dyn PackageProcessor + Send + Sync + 'static>,
            }
        })
    }
}

struct CloneableProcessor<'a>(&'a (dyn PackageProcessor + Send + Sync + 'static));

#[async_trait]
impl<'a> PackageProcessor for CloneableProcessor<'a> {
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()> {
        self.0.process(url, output_dir, config).await
    }
    
    fn name(&self) -> &'static str {
        self.0.name()
    }
    
    fn accepts(&self, url: &str) -> bool {
        self.0.accepts(url)
    }
    
    async fn validate(&self, url: &str) -> Result<()> {
        self.0.validate(url).await
    }
} 