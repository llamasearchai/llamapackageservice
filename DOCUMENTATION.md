# LlamaPackageService API Documentation

## Table of Contents

- [Introduction](#introduction)
- [Core Components](#core-components)
  - [Config](#config)
  - [Processors](#processors)
  - [Cache System](#cache-system)
  - [Error Handling](#error-handling)
  - [User Interface](#user-interface)
- [API Reference](#api-reference)
  - [Package Processor Trait](#package-processor-trait)
  - [GitHub Processor](#github-processor)
  - [PyPI Processor](#pypi-processor)
  - [NPM Processor](#npm-processor)
  - [Crates Processor](#crates-processor)
- [Usage Examples](#usage-examples)
- [Best Practices](#best-practices)

## Introduction

LlamaPackageService is a modular Rust application designed to process code repositories from various sources and convert them into structured text representations. This document provides detailed information about the API, core components, and usage patterns.

## Core Components

### Config

The `Config` struct serves as the central configuration for the application, managing settings related to output directories, API tokens, rate limits, and processing parameters.

```rust
pub struct Config {
    pub github_token: Option<String>,
    pub output_dir: PathBuf,
    pub processing: ProcessingConfig,
    pub rate_limits: RateLimits,
    pub output_config: OutputConfig,
    pub api_keys: ApiKeys,
    pub excluded_files: Vec<String>,
}
```

Key features:
- **Environment Integration**: Automatically reads tokens from environment variables
- **Validation**: Ensures directories exist and tokens are properly configured
- **Default Configurations**: Sensible defaults for all settings
- **File Pattern Exclusion**: Regex-based file exclusion patterns

### Processors

The processor modules handle specific package sources and implement the `PackageProcessor` trait. Each processor is responsible for:

1. Validating repository URLs
2. Downloading repository content
3. Extracting and parsing relevant information
4. Generating structured text output

Available processors:
- **GitHub Processor**: Handles GitHub repositories
- **PyPI Processor**: Processes Python packages
- **NPM Processor**: Handles Node.js packages
- **Crates Processor**: Processes Rust crates

### Cache System

LlamaPackageService implements a sophisticated caching system to optimize performance and reduce redundant operations.

Key components:
- **FileCache**: Caches downloaded files to disk with configurable TTL
- **Cache<T>**: Generic in-memory cache for any type with time-based expiration
- **StringCache**: Specialized cache for string values with disk persistence

```rust
pub struct FileCache {
    cache_dir: PathBuf,
    default_ttl: Duration,
}

pub struct Cache<T> {
    store: Arc<RwLock<HashMap<String, (T, Instant)>>>,
    ttl: Duration,
}

pub struct StringCache {
    store: Arc<RwLock<HashMap<String, (String, Instant)>>>,
    ttl: Duration,
    cache_dir: PathBuf,
}
```

### Error Handling

The error handling system uses Rust's `thiserror` to provide comprehensive error information with proper context.

```rust
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    // Additional error variants...
}
```

Features:
- **Error Classification**: Errors are categorized as transient or fatal
- **Context Preservation**: Full error context is maintained throughout the call stack
- **Automatic Conversion**: From standard library errors to application-specific errors
- **Human-Readable Messages**: Clear error messages for better debugging

### User Interface

The `LlamaUI` struct provides an interactive terminal interface with:

- **Main Menu**: Easy navigation through application features
- **Progress Indicators**: Real-time progress bars for long-running operations
- **Colorful Output**: Color-coded messages for different information types
- **Input Prompts**: User-friendly input collection

## API Reference

### Package Processor Trait

The `PackageProcessor` trait defines the interface that all package processors must implement:

```rust
#[async_trait]
pub trait PackageProcessor: Send + Sync {
    /// Get the name of the processor
    fn name(&self) -> &'static str;
    
    /// Check if this processor accepts the given URL
    fn accepts(&self, url: &str) -> bool;
    
    /// Process a package from a URL
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()>;

    /// Validate if a URL is supported by this processor
    async fn validate(&self, url: &str) -> Result<()>;
}
```

### GitHub Processor

The `GitHubProcessor` handles GitHub repositories, extracting information like:
- Repository metadata (stars, forks, description)
- README content
- File structure
- Dependencies
- Code organization

Key methods:
- `process`: Main entrypoint for processing a GitHub repository
- `download_repo`: Downloads a repository archive
- `extract_repo_info`: Extracts repository metadata
- `process_repo_contents`: Processes the repository files

### PyPI Processor

The `PyPiProcessor` handles Python packages from PyPI, extracting:
- Package metadata
- Installation instructions
- Dependencies
- Documentation links
- Author information

```rust
pub struct PyPiProcessor {
    client: Client,
}

impl PyPiProcessor {
    pub fn new() -> Self { /* ... */ }
    
    async fn validate_pypi_package(&self, package_name: &str) -> Result<()> { /* ... */ }
    
    async fn extract_package_name(&self, url: &str) -> Result<String> { /* ... */ }
    
    async fn get_package_info(&self, package_name: &str) -> Result<Value> { /* ... */ }
    
    async fn process_package(&self, package_name: &str, output_dir: &Path) -> Result<()> { /* ... */ }
}
```

### NPM Processor

The `NpmProcessor` handles Node.js packages, extracting:
- Package.json information
- Dependencies and versions
- Script definitions
- Repository links
- Author and maintainer data

### Crates Processor

The `CratesProcessor` handles Rust crates from crates.io, extracting:
- Crate metadata
- Cargo.toml dependencies
- Documentation links
- Repository structure
- Version history

## Usage Examples

### Basic Repository Processing

```rust
use llamapackageservice::{Config, GitHubProcessor, PackageProcessor};
use std::path::Path;

async fn process_github_repo() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let processor = GitHubProcessor::new();
    
    processor.process(
        "https://github.com/rust-lang/rust", 
        Path::new("output"), 
        &config
    ).await?;
    
    Ok(())
}
```

### Customized Configuration

```rust
use llamapackageservice::{Config, GitHubProcessor, PackageProcessor};
use std::path::PathBuf;
use std::time::Duration;

async fn process_with_custom_config() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new(PathBuf::from("custom_output"));
    
    // Set GitHub token
    config.github_token = std::env::var("GITHUB_TOKEN").ok();
    
    // Customize processing limits
    config.processing.max_concurrent_downloads = 3;
    config.processing.max_concurrent_extractions = 2;
    
    // Set custom rate limits
    config.rate_limits.github_api = 3000;
    
    // Customize output configuration
    config.output_config.cache_duration = Duration::from_secs(7200); // 2 hours
    
    // Add excluded file patterns
    config.excluded_files.push(r"\.git/".to_string());
    config.excluded_files.push(r"node_modules/".to_string());
    
    let processor = GitHubProcessor::new();
    processor.process(
        "https://github.com/example/repo", 
        &config.output_dir, 
        &config
    ).await?;
    
    Ok(())
}
```

### Processor Factory

```rust
use llamapackageservice::{Config, processors::{ProcessorFactory}};
use std::path::Path;

async fn process_any_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    
    // Create appropriate processor based on URL
    let processor = ProcessorFactory::create_processor(url)?;
    
    // Process the URL
    processor.process(url, Path::new("output"), &config).await?;
    
    Ok(())
}
```

## Best Practices

1. **Token Management**: Store API tokens securely using environment variables
2. **Error Handling**: Always handle errors appropriately, especially network and IO errors
3. **Rate Limiting**: Respect API rate limits by using built-in rate limiting features
4. **Caching**: Leverage the caching system to improve performance
5. **Concurrency Control**: Use the processing configuration to balance system load
6. **Output Directory Management**: Create dedicated output directories for different types of repositories
7. **Progress Reporting**: Use the UI progress indicators for long-running operations 