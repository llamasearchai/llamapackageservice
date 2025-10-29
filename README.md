# LlamaPackageService

<div align="center">

![LlamaPackageService Logo](assets/logo.png)

**Convert code repositories into structured text for analysis and exploration**

[![Rust](https://img.shields.io/badge/Rust-1.76.0%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/yourusername/llamapackageservice/rust.yml?branch=main)](https://github.com/yourusername/llamapackageservice/actions)
[![Docs](https://img.shields.io/badge/docs-latest-brightgreen.svg)](https://docs.rs/llamapackageservice)

</div>

## Overview

LlamaPackageService is a high-performance tool designed to transform code repositories from various sources (GitHub, PyPI, NPM, Crates.io) into structured text representations. This enables easier integration with language models, code analysis tools, and search engines.

With a focus on reliability, performance, and user experience, LlamaPackageService provides both a programmer-friendly API and an interactive terminal interface.

## Features

- **Multiple Package Sources**: Process repositories from GitHub, PyPI packages, NPM packages, and Rust crates
- **Intelligent Extraction**: Parse and extract meaningful information from code, documentation, and metadata
- **Caching System**: Efficient caching to avoid redundant downloads and processing
- **Rate Limiting**: Built-in rate limiting to respect API usage policies
- **Parallel Processing**: Optimized concurrent operations for faster processing
- **Interactive UI**: Llama-themed terminal user interface with progress indicators
- **Comprehensive Error Handling**: Robust error handling with informative error messages

## Installation

### From Cargo

```bash
cargo install llamapackageservice
```

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/llamapackageservice.git
cd llamapackageservice

# Build the project
cargo build --release

# Run the binary
./target/release/llamapackageservice
```

## Quick Start

### Command Line Interface

```bash
# Process a GitHub repository
llamapackageservice --url https://github.com/username/repo

# Process a PyPI package
llamapackageservice --url https://pypi.org/project/requests/

# Specify output directory
llamapackageservice --url https://github.com/username/repo --output ./my_output_dir

# Run in interactive mode
llamapackageservice --interactive
```

### Library Usage

```rust,ignore
use llamapackageservice::{Config, GitHubProcessor, PackageProcessor};
use std::path::Path;

async fn process_repository() -> Result<(), Box<dyn std::error::Error>> {
    // Create default configuration
    let config = Config::default();
    
    // Initialize GitHub processor
    let processor = GitHubProcessor::new();
    
    // Process a repository
    processor.process(
        "https://github.com/rust-lang/rust", 
        Path::new("output"), 
        &config
    ).await?;
    
    Ok(())
}
```

## Architecture

LlamaPackageService is built around a modular processor architecture:

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│                 │     │                 │     │                 │
│  User Interface │────▶│    Processors   │────▶│  Output Engine  │
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                        ┌──────┴──────┐
                        ▼             ▼
                   ┌─────────┐  ┌────────────┐
                   │ Caching │  │Rate Limiter│
                   └─────────┘  └────────────┘
```

- **User Interface**: Handles CLI arguments and interactive terminal UI
- **Processors**: Specialized modules for each supported package source
- **Output Engine**: Formats and saves processed content
- **Caching**: Improves performance by storing frequently accessed data
- **Rate Limiter**: Ensures API usage remains within acceptable limits

## Documentation

For comprehensive documentation, visit:
- [API Reference](https://docs.rs/llamapackageservice)
- [User Guide](https://github.com/yourusername/llamapackageservice/wiki)
- [Architecture Overview](https://github.com/yourusername/llamapackageservice/wiki/Architecture)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- The Rust community for their amazing crates
- All contributors who have helped shape this project
