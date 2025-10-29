# LlamaPackageService User Guide

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Command Line Interface](#command-line-interface)
- [Interactive Mode](#interactive-mode)
- [Configuration](#configuration)
- [API Tokens](#api-tokens)
- [Processing Sources](#processing-sources)
  - [GitHub Repositories](#github-repositories)
  - [PyPI Packages](#pypi-packages)
  - [NPM Packages](#npm-packages)
  - [Rust Crates](#rust-crates)
- [Output Formats](#output-formats)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)
- [Frequently Asked Questions](#frequently-asked-questions)

## Introduction

LlamaPackageService is a powerful tool designed to transform code repositories from various sources (GitHub, PyPI, NPM, Crates.io) into structured text representations. This allows for easier integration with language models, code analysis tools, and search engines.

This guide will walk you through the installation, configuration, and usage of LlamaPackageService to get the most out of its features.

## Installation

### From Cargo (Recommended)

The easiest way to install LlamaPackageService is via Cargo:

```bash
cargo install llamapackageservice
```

### From Source

If you prefer to build from source:

```bash
# Clone the repository
git clone https://github.com/yourusername/llamapackageservice.git
cd llamapackageservice

# Build the project
cargo build --release

# Run the binary
./target/release/llamapackageservice
```

### Dependencies

LlamaPackageService requires:
- Rust 1.65 or newer
- Internet connection for downloading repositories
- Optional: API tokens for higher rate limits

## Quick Start

### Process a GitHub Repository

```bash
llamapackageservice --url https://github.com/username/repo
```

### Process a PyPI Package

```bash
llamapackageservice --url https://pypi.org/project/requests/
```

### Specify Output Directory

```bash
llamapackageservice --url https://github.com/username/repo --output ./my_output_dir
```

### Run in Interactive Mode

```bash
llamapackageservice --interactive
```

## Command Line Interface

LlamaPackageService provides a comprehensive command-line interface:

### Basic Usage

```
USAGE:
    llamapackageservice [OPTIONS] [COMMAND]

OPTIONS:
    -u, --url <URL>                 URL of the repository or package to process
    -o, --output <DIRECTORY>        Output directory [default: ./output]
    -i, --interactive               Start in interactive mode
    -c, --config <FILE>             Path to configuration file
    -v, --verbose                   Enable verbose output
    -h, --help                      Print help information
    -V, --version                   Print version information

COMMANDS:
    github      Process a GitHub repository
    pypi        Process a PyPI package
    npm         Process an NPM package
    crates      Process a Rust crate
    help        Print this message or help for a specific command
```

### Command Examples

#### GitHub Command

```bash
llamapackageservice github --repo username/repo --branch main
```

#### PyPI Command

```bash
llamapackageservice pypi --package requests --version latest
```

#### NPM Command

```bash
llamapackageservice npm --package express --version 4.17.1
```

#### Crates Command

```bash
llamapackageservice crates --package tokio --version 1.0.0
```

## Interactive Mode

LlamaPackageService features an interactive terminal UI that guides you through the process:

1. Start interactive mode:
   ```bash
   llamapackageservice --interactive
   ```

2. Navigate the main menu using arrow keys or by entering the option number.

3. Available options in the interactive menu:
   - Process GitHub Repository
   - Process PyPI Package
   - Process NPM Package
   - Process Crates.io Package
   - Configure Settings
   - View Recent Processes
   - Exit

4. Follow the on-screen prompts to enter necessary information like repository URLs.

5. View real-time progress indicators during processing.

## Configuration

LlamaPackageService can be configured through a configuration file, environment variables, or command-line arguments.

### Configuration File

Create a `llamapackageservice.toml` file in one of these locations:
- Current directory
- `$HOME/.config/llamapackageservice/`
- `/etc/llamapackageservice/`

Example configuration:

```toml
[general]
output_dir = "/path/to/output"
default_processor = "github"

[api]
github_token = "your_github_token"
pypi_token = "your_pypi_token"

[rate_limits]
github_api = 5000
pypi_api = 100

[processing]
max_concurrent_downloads = 5
max_concurrent_extractions = 3

[output]
format = "text"
cache_duration = 7200

[exclude]
files = [
    ".*\\.git/.*",
    ".*node_modules/.*",
    ".*\\.png$",
    ".*\\.jpg$"
]
```

### Environment Variables

You can also use environment variables for configuration:

```bash
# API Tokens
export LLAMAPACKAGESERVICE_GITHUB_TOKEN="your_github_token"
export LLAMAPACKAGESERVICE_PYPI_TOKEN="your_pypi_token"

# Output Directory
export LLAMAPACKAGESERVICE_OUTPUT_DIR="/path/to/output"

# Processing Configuration
export LLAMAPACKAGESERVICE_MAX_CONCURRENT_DOWNLOADS="5"
```

## API Tokens

For higher rate limits and access to private repositories, LlamaPackageService can use API tokens:

### GitHub Token

1. Create a [GitHub Personal Access Token](https://github.com/settings/tokens)
2. Set it in one of these ways:
   - Environment variable: `LLAMAPACKAGESERVICE_GITHUB_TOKEN`
   - Configuration file: `github_token` under `[api]` section
   - Command line: `--github-token <TOKEN>`

### PyPI Token

For PyPI access, set your token:
- Environment variable: `LLAMAPACKAGESERVICE_PYPI_TOKEN`
- Configuration file: `pypi_token` under `[api]` section

## Processing Sources

LlamaPackageService supports multiple package sources:

### GitHub Repositories

Process any public or private (with token) GitHub repository:

```bash
llamapackageservice --url https://github.com/username/repo
```

Options:
- `--branch`: Specify the branch to process (default: main/master)
- `--commit`: Specify a commit hash to process
- `--depth`: Limit the clone depth (default: 1)

### PyPI Packages

Process Python packages from PyPI:

```bash
llamapackageservice --url https://pypi.org/project/requests/
```

Options:
- `--version`: Specify package version (default: latest)
- `--include-deps`: Include dependencies (default: false)

### NPM Packages

Process JavaScript packages from NPM:

```bash
llamapackageservice --url https://www.npmjs.com/package/express
```

Options:
- `--version`: Specify package version (default: latest)
- `--include-deps`: Include dependencies (default: false)

### Rust Crates

Process Rust crates from crates.io:

```bash
llamapackageservice --url https://crates.io/crates/tokio
```

Options:
- `--version`: Specify crate version (default: latest)
- `--include-deps`: Include dependencies (default: false)

## Output Formats

LlamaPackageService generates structured output:

### Directory Structure

```
output/
├── metadata.json         # Package/repository metadata
├── summary.txt           # Overall summary
├── dependencies.json     # Dependency information
├── contents/
│   ├── src/              # Source files
│   ├── docs/             # Documentation
│   └── ...
└── analysis/             # Additional analysis data
```

### Customizing Output

You can customize output using the configuration file:

```toml
[output]
# Output format (text, json, markdown)
format = "text"

# Include source code in output
include_source = true

# Include tests in output
include_tests = true

# Generate dependency graph
generate_graph = true
```

## Performance Tuning

Optimize LlamaPackageService for your system:

### Concurrent Operations

Adjust the number of concurrent operations:

```toml
[processing]
# Maximum concurrent downloads
max_concurrent_downloads = 5

# Maximum concurrent extractions
max_concurrent_extractions = 3
```

### Caching

Configure the caching behavior:

```toml
[output]
# Cache duration in seconds (default: 3600)
cache_duration = 7200

# Maximum cache size in MB (default: 1000)
max_cache_size = 2000
```

### File Exclusions

Exclude unnecessary files to improve performance:

```toml
[exclude]
files = [
    ".*\\.git/.*",
    ".*node_modules/.*",
    ".*\\.png$",
    ".*\\.jpg$"
]
```

## Troubleshooting

### Common Issues

#### Rate Limiting

If you encounter rate limiting issues:
- Use API tokens for higher limits
- Reduce concurrent operations
- Enable automatic retries

```toml
[rate_limits]
# Enable automatic retries on rate limit
auto_retry = true

# Maximum retry attempts
max_retries = 3

# Delay between retries (seconds)
retry_delay = 60
```

#### Permission Errors

If you see permission errors when creating output files:
- Check the output directory permissions
- Verify that you have write access to the specified location
- Try using a different output location

#### Network Errors

For network-related issues:
- Check your internet connection
- Verify that the repository URL is correct and accessible
- Consider using a proxy if your network restricts direct access

```toml
[network]
# HTTP proxy URL
proxy = "http://your-proxy:8080"

# Timeout in seconds
timeout = 30
```

### Diagnostic Information

Enable verbose logging for troubleshooting:

```bash
llamapackageservice --verbose --url https://github.com/username/repo
```

For even more detailed logs:

```bash
RUST_LOG=debug llamapackageservice --url https://github.com/username/repo
```

## Frequently Asked Questions

### General Questions

**Q: How much disk space does LlamaPackageService require?**

A: The amount of disk space depends on the repositories you process. Each processed repository typically requires:
- Space for the downloaded repository
- Space for the output files (usually smaller than the repository)
- Cache space (configurable)

**Q: Can LlamaPackageService process private repositories?**

A: Yes, with the appropriate API tokens. For GitHub, you need a personal access token with the `repo` scope.

**Q: How are large repositories handled?**

A: LlamaPackageService uses streaming and incremental processing for large repositories. You can also adjust the `max_file_size` setting to skip extremely large files.

### Technical Questions

**Q: Can I use LlamaPackageService as a library in my Rust project?**

A: Yes, LlamaPackageService can be used as a library:

```rust
use llamapackageservice::{Config, GitHubProcessor, PackageProcessor};
use std::path::Path;

async fn process_repo() -> Result<(), Box<dyn std::error::Error>> {
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

**Q: Does LlamaPackageService support custom processors?**

A: Yes, you can implement the `PackageProcessor` trait to create custom processors for different package sources:

```rust
#[async_trait]
impl PackageProcessor for MyCustomProcessor {
    fn name(&self) -> &'static str {
        "my_custom_processor"
    }
    
    fn accepts(&self, url: &str) -> bool {
        url.contains("mycustomsource.com")
    }
    
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()> {
        // Implementation
    }
    
    async fn validate(&self, url: &str) -> Result<()> {
        // Implementation
    }
}
``` 