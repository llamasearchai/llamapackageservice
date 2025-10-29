use async_trait::async_trait;
use indicatif::ProgressBar;
use std::path::Path;
use std::error::Error;

/// The core trait for package processing tasks.
/// It defines a method that performs a processing routine.
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, output_dir: &Path, pb: &ProgressBar) -> Result<(), Box<dyn Error + Send + Sync>>;
} 