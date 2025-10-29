#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

//! LlamaPackageService - A comprehensive package processing and analysis tool
//!
//! This library provides functionality to process and analyze code repositories from various sources
//! including GitHub, PyPI, NPM registries, and local directories. It can generate comprehensive
//! documentation, perform security audits, and provide AI-powered code analysis.
//!
//! ## Features
//! - Process GitHub repositories and organizations
//! - Process PyPI packages
//! - Process NPM packages
//! - Process Rust crates
//! - Analytics and metrics collection
//! - Caching for improved performance
//!
//! ## Usage
//! ```rust,ignore
//! use llamapackageservice::{Config, GitHubProcessor, PackageProcessor};
//! use std::path::Path;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::default();
//!     let processor = GitHubProcessor::new();
//!     
//!     processor.process("https://github.com/rust-lang/rust", 
//!                    Path::new("output"), 
//!                    &config).await?;
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use std::{env, path::Path};

/// Configuration module for the application
pub mod config;
/// Caching mechanisms to improve performance
pub mod cache;
/// Error handling types and utilities
pub mod error;
/// Logging configuration and utilities
pub mod logging;
/// Metrics collection and reporting
pub mod metrics;
/// Package processors for different sources (GitHub, PyPI, NPM, etc.)
pub mod processors;
/// Parallel processing utilities
pub mod parallel;
/// Rate limiting functionality to respect API limits
pub mod rate_limiter;
/// Output organization and indexing utilities
pub mod output_organizer;
/// REST API functionality for web service
pub mod api;
/// Agents for AI-powered analysis
pub mod agents;
/// Utilities (path normalization, retry helpers, cache helpers)
pub mod utils;

// Re-export common types
pub use config::Config;
pub use error::{ProcessorError, Result};
pub use processors::github::GitHubProcessor;
pub use processors::pypi::PyPiProcessor;
pub use processors::PackageProcessor as ProcessorTrait;
pub use output_organizer::{list_output_files, organize_output, generate_index};
pub use agents::{OpenAIAgent, AnalysisRequest, AnalysisResult, AnalysisType};

/// A trait for package processors that can handle different types of package sources
#[allow(async_fn_in_trait)]
pub trait PackageProcessor: Send + Sync + Clone {
    /// Get the name of the processor
    fn name(&self) -> &'static str;
    
    /// Check if this processor accepts the given URL
    fn accepts(&self, url: &str) -> bool;
    
    /// Process a package from a URL.
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> error::Result<()>;

    /// Validate if a URL is supported by this processor.
    async fn validate(&self, url: &str) -> error::Result<()>;
}

/// Output paths configuration for managing file storage locations
#[derive(Clone, Debug)]
pub struct OutputPaths {
    /// Base directory for all output files
    pub base_dir: std::path::PathBuf,
    /// Temporary directory for processing files
    pub temp_dir: std::path::PathBuf,
}

/// Limits memory usage by setting environment variables for stack size
pub fn limit_memory_usage() {
    // Set reasonable stack size limits - can be adjusted as needed
    let _ = env::set_var("RUST_MIN_STACK", "4194304"); // 4 MB
}