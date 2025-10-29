use crate::processor_trait::Processor;
use crate::error::ProcessorError;
use async_trait::async_trait;
use indicatif::ProgressBar;
use std::{error::Error as StdError, path::Path};
use time::OffsetDateTime;
use crate::ui::ProcessorResult;
use crate::OutputPaths;
use std::sync::Arc;
use tokio;

/// A simple processor that simulates a package processing workflow.
pub struct SimpleProcessor {
    pub package_name: String,
}

impl SimpleProcessor {
    pub fn new(name: String) -> Self {
        SimpleProcessor { package_name: name }
    }
}

// Rename the struct to avoid conflict with the trait
pub struct BasicPackageProcessor {
    pub name: String,
    pub source_url: String,
    pub processor_type: String,
}

#[async_trait]
impl Processor for BasicPackageProcessor {
    async fn process(&self, output_dir: &Path, pb: &ProgressBar) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let msg = format!("Processing {} package: {}", self.processor_type, self.name);
        pb.set_message(msg);
        
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let file_name = format!("{}_{}.txt", self.name, timestamp);
        let file_path = output_dir.join(file_name);
        
        let content = format!(
            "Package: {}\nType: {}\nSource: {}\nProcessed: {}\n",
            self.name,
            self.processor_type,
            self.source_url,
            OffsetDateTime::now_utc()
        );
        
        tokio::fs::write(&file_path, content).await?;
        
        pb.finish_with_message(format!("{} package {} processed successfully", self.processor_type, self.name));
        Ok(())
    }
}

pub async fn process_package(
    name: &str,
    source_url: &str,
    processor_type: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<(), ProcessorError> {
    let processor = BasicPackageProcessor {
        name: name.to_string(),
        source_url: source_url.to_string(),
        processor_type: processor_type.to_string(),
    };
    
    processor
        .process(output_dir, pb)
        .await
        .map_err(|e| ProcessorError::Processing(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_simple_processor() {
        let temp_dir = TempDir::new().unwrap();
        let processor = SimpleProcessor::new("test_package".to_string());
        let pb = ProgressBar::new(0);
        
        // Test with actual directory and progress bar
        assert!(processor.process(temp_dir.path(), &pb).await.is_ok());
    }
}

// Define the trait for package processors
#[async_trait]
pub trait PackageProcessor: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn accepts(&self, url: &str) -> bool;
    async fn validate(&self, url: &str) -> Result<(), Box<dyn StdError + Send + Sync>>;
    async fn process(
        self: Arc<Self>,
        url: &str,
        paths: &OutputPaths,
        pb: &ProgressBar,
    ) -> ProcessorResult<()>;
}

#[async_trait]
impl Processor for SimpleProcessor {
    async fn process(&self, _output_dir: &Path, pb: &ProgressBar) -> Result<(), Box<dyn StdError + Send + Sync>> {
        pb.set_message(format!("Processing package: {}", self.package_name));
        // Simulate processing
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        pb.finish_with_message("Package processed successfully!");
        Ok(())
    }
} 