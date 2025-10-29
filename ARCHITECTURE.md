# LlamaPackageService Architecture

## System Overview

LlamaPackageService is designed around a modular, maintainable architecture that enables processing code repositories from multiple sources and converting them into structured text representations. The system follows Rust best practices including error handling, concurrency, and performance optimization.

![Architecture Diagram](assets/architecture.png)

## Core Design Principles

1. **Separation of Concerns**: Each module has a clear responsibility
2. **Modularity**: Components are designed to be easily replaceable and testable
3. **Error Propagation**: Comprehensive error handling throughout the system
4. **Performance Optimization**: Efficient algorithms and resource usage
5. **Concurrent Processing**: Parallel execution where appropriate
6. **User Experience**: Intuitive interfaces both for CLI and library users

## Layer Structure

The application is organized into several logical layers:

### 1. User Interface Layer

The UI layer provides both a CLI interface and interactive terminal UI:

* `main.rs`: Entry point for CLI arguments parsing
* `llama_ui.rs`: Interactive terminal UI with Llama-themed design
* `commands.rs`: Command definitions and processors

### 2. Processor Layer

The processor layer contains specialized modules for different package sources:

* `processors/mod.rs`: Defines the `PackageProcessor` trait
* `processors/github.rs`: GitHub repository processor
* `processors/pypi.rs`: Python Package Index processor
* `processors/npm.rs`: Node Package Manager processor
* `processors/crates.rs`: Crates.io processor
* `processors/registry.rs`: Registry for all processors
* `processors/common.rs`: Shared utilities and functions

### 3. Infrastructure Layer

The infrastructure layer provides essential services:

* `cache.rs`: Caching system for files and metadata
* `rate_limiter.rs`: Rate limiting for API requests
* `error.rs`: Error types and handling
* `logging.rs`: Logging infrastructure
* `metrics.rs`: Performance and usage metrics
* `config/`: Configuration management

### 4. Core Utilities

Utilities for various operations:

* `parallel.rs`: Utilities for parallel processing
* `utils/`: Miscellaneous utility functions
* `constants.rs`: System-wide constants

## Key Components

### Package Processor Implementation

The core abstraction of the system is the `PackageProcessor` trait:

```rust
#[async_trait]
pub trait PackageProcessor: Send + Sync {
    fn name(&self) -> &'static str;
    fn accepts(&self, url: &str) -> bool;
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()>;
    async fn validate(&self, url: &str) -> Result<()>;
}
```

Each processor implementation must:

1. Identify if it can handle a specific URL
2. Validate the URL format
3. Process the repository/package
4. Generate structured output

### Concurrency Model

LlamaPackageService uses Rust's async/await for concurrency, with Tokio as the runtime:

```rust
pub struct ParallelExecutor {
    workers: usize,
}

impl ParallelExecutor {
    pub fn new(workers: usize) -> Self {
        // Initialize with configurable worker count
    }

    pub async fn execute<F, Fut, T>(&self, tasks: Vec<F>) -> Vec<Result<T>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        // Process tasks concurrently with controlled parallelism
    }
}
```

This enables:
- Controlled concurrent downloads
- Parallel file processing
- Responsive UI during long operations
- Backpressure control

### Caching System

The caching system uses a tiered approach:

1. **In-memory cache**: Fast access for frequently used items
2. **File cache**: Persistent storage for downloaded artifacts
3. **Metadata cache**: Storage for repository and package metadata

```rust
pub struct Cache<T> {
    store: Arc<RwLock<HashMap<String, (T, Instant)>>>,
    ttl: Duration,
}

impl<T: Clone + Send + Sync + 'static> Cache<T> {
    // Cache implementation with expiry
}
```

The cache implements:
- Time-based expiration
- Thread-safe access
- Configurable TTL (Time To Live)
- Automatic pruning of expired entries

### Error Handling Architecture

The error system uses `thiserror` to create a comprehensive error hierarchy:

```rust
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Package not found: {package}")]
    PackageNotFound { package: String },
    
    // Additional error variants
}
```

Error handling features:
- Context preservation with `anyhow`
- Error classification (fatal vs. transient)
- Automatic conversion between error types
- Clear error messages

### Terminal UI Implementation

The terminal UI uses the `dialoguer` and `console` crates to create an interactive experience:

```rust
pub struct LlamaUI {
    term: Term,
}

impl LlamaUI {
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
        }
    }

    pub fn show_main_menu(&self, items: &[LlamaMenuOption]) -> Result<LlamaMenuOption> {
        // Interactive menu implementation
    }

    pub fn create_progress_bar(&self, length: u64) -> ProgressBar {
        // Progress bar creation
    }
}
```

The UI provides:
- Interactive menus
- Progress bars for long operations
- Color-coded output
- Error messages
- Input collection

## Data Flow

### Repository Processing Flow

1. **URL Validation**: Check if the URL is valid and determine the processor
2. **Metadata Retrieval**: Fetch repository/package metadata
3. **Content Download**: Download the repository/package
4. **Content Extraction**: Extract files from archives
5. **Content Processing**: Process files into structured text
6. **Output Generation**: Generate the final output in the desired format

```
┌─────────┐     ┌─────────┐     ┌─────────────┐     ┌─────────┐     ┌─────────┐
│  Input  │────▶│Validate │────▶│  Download   │────▶│ Extract │────▶│ Process │
└─────────┘     └─────────┘     └─────────────┘     └─────────┘     └─────────┘
                                       │                                 │
                                       ▼                                 ▼
                                  ┌─────────┐                      ┌─────────┐
                                  │  Cache  │                      │ Output  │
                                  └─────────┘                      └─────────┘
```

### Caching Flow

1. **Request**: Application requests a resource
2. **Cache Check**: Check if the resource is in the cache
3. **Cache Hit**: Return cached resource if valid
4. **Cache Miss**: Fetch the resource and store in cache
5. **Return**: Return the resource to the requester

```
┌─────────┐     ┌─────────┐     ┌─────────────┐
│ Request │────▶│  Check  │────▶│ Cache Hit?  │
└─────────┘     └─────────┘     └─────────────┘
                                       │
                                       ├──Yes──▶┌─────────┐
                                       │         │ Return  │
                                       │         │ Cached  │
                                       │         └─────────┘
                                       │
                                       └──No───▶┌─────────┐     ┌─────────┐
                                                │  Fetch  │────▶│  Store  │
                                                └─────────┘     └─────────┘
                                                                     │
                                                                     ▼
                                                               ┌─────────┐
                                                               │ Return  │
                                                               │  New    │
                                                               └─────────┘
```

## Configuration Management

LlamaPackageService uses a hierarchical configuration system:

1. **Default Values**: Sensible defaults for all settings
2. **Environment Variables**: Override defaults with environment variables
3. **Command-Line Arguments**: Override environment variables with CLI args
4. **Configuration File**: Optional config file for persistent settings

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

impl Config {
    pub fn from_env() -> Self {
        // Load configuration from environment
    }

    pub fn default() -> Self {
        // Create default configuration
    }

    pub fn with_output_dir(output_dir: PathBuf) -> Self {
        // Create config with custom output dir
    }
}
```

## Performance Considerations

LlamaPackageService is designed with performance in mind:

1. **Parallel Processing**: Controlled concurrency for I/O-bound operations
2. **Efficient Memory Usage**: Streaming downloads to avoid excessive memory use
3. **Caching Layers**: Multi-tiered caching to reduce redundant operations
4. **Rate Limiting**: Smart rate limiting to maximize throughput within API constraints
5. **Backpressure Control**: Prevent resource exhaustion during heavy loads
6. **Early Filtering**: Filter unwanted files before full processing
7. **Zero-Copy Parsing**: Efficient parsing of downloaded content

## Security Considerations

Security measures implemented in the system:

1. **Token Management**: Secure handling of API tokens
2. **Input Validation**: Thorough validation of all inputs
3. **Path Traversal Protection**: Prevention of path traversal attacks
4. **Temporary File Handling**: Secure management of temporary files
5. **Rate Limiting**: Protection against excessive API usage
6. **Error Information**: Careful control of error information exposure

## Testing Strategy

The codebase follows a comprehensive testing strategy:

1. **Unit Tests**: Testing individual components in isolation
2. **Integration Tests**: Testing component interactions
3. **Mock Testing**: Using mocks for external dependencies
4. **Snapshot Testing**: Validating output consistency
5. **Property Testing**: Using property-based testing for complex logic
6. **CI Integration**: Automated testing on each commit

## Future Extensibility

The architecture is designed to be extensible:

1. **New Processors**: Easy addition of new package sources
2. **Output Formats**: Support for different output formats
3. **Plugins**: Potential plugin system for custom processing
4. **API Integration**: Possible REST API for remote processing
5. **Cloud Integration**: Potential cloud service integration

## Appendix: Directory Structure

```
llamapackageservice/
├── src/
│   ├── main.rs               # Application entry point
│   ├── llama_ui.rs           # Terminal UI implementation
│   ├── cache.rs              # Caching system
│   │   ├── mod.rs            # Configuration module
│   │   └── env_manager.rs    # Environment variable management
│   ├── error.rs              # Error types and handling
│   ├── logging.rs            # Logging infrastructure
│   ├── metrics.rs            # Performance and usage metrics
│   ├── parallel.rs           # Parallel processing utilities
│   ├── rate_limiter.rs       # API rate limiting
│   ├── processors/
│   │   ├── mod.rs            # Processor trait and registry
│   │   ├── github.rs         # GitHub processor
│   │   ├── pypi.rs           # PyPI processor
│   │   ├── npm.rs            # NPM processor
│   │   ├── crates.rs         # Crates.io processor
│   │   ├── registry.rs       # Processor registry
│   │   └── common.rs         # Common processor utilities
│   └── utils/
│       ├── mod.rs            # Utility module
│       └── fs.rs             # Filesystem utilities
├── tests/
│   ├── integration_tests/    # Integration tests
│   └── unit_tests/           # Unit tests
├── examples/                  # Example code
├── benches/                   # Benchmarks
├── docs/                      # Documentation
└── assets/                    # Assets (images, etc.)
``` 