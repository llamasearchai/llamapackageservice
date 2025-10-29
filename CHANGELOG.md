# Changelog

All notable changes to the LlamaPackageService project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2023-06-15

### Added
- Full documentation with API references and user guides
- CI/CD pipeline with GitHub Actions
- Code quality checks and linting rules
- Interactive terminal user interface
- Performance monitoring and metrics collection
- Comprehensive test suite with unit and integration tests

### Changed
- Renamed project from "code2txt" to "LlamaPackageService"
- Reworked the terminal UI for improved user experience
- Redesigned configuration system for more flexibility

### Fixed
- Terminal height handling to prevent UI crashes
- Rate limiting issues with the GitHub API
- Memory usage optimizations for large repositories

## [0.9.0] - 2023-05-20

### Added
- Dependency analysis for all supported package types
- Content summarization using NLP techniques
- Performance optimizations for large repositories
- Advanced caching strategy for improved speed
- User configuration profiles

### Changed
- Improved parallel processing for faster execution
- Enhanced error reporting with detailed context
- Better handling of rate limits across all APIs

### Fixed
- Edge case handling for uncommon repository structures
- Memory leaks in long-running processing tasks
- Race conditions in concurrent download operations

## [0.8.0] - 2023-04-10

### Added
- Support for Rust crates from crates.io
- Progress indication for long-running operations
- Comprehensive logging system
- Export functionality for processed repositories
- Integration with external analysis tools

### Changed
- Enhanced error handling with clearer messages
- Improved file exclusion patterns
- More efficient handling of large file downloads

### Fixed
- Repository URL parsing issues
- Cache invalidation bugs
- Output formatting inconsistencies

## [0.7.0] - 2023-03-05

### Added
- NPM package processor implementation
- Full test suite for all processors
- User-friendly command-line interface
- Documentation for API usage
- Performance benchmarks

### Changed
- Reorganized processor code for better maintainability
- Enhanced metadata extraction for all package types
- Improved error messages and debug information

### Fixed
- PyPI API handling for large packages
- HTTP client connection pooling issues
- Path handling on different operating systems

## [0.6.0] - 2023-02-15

### Added
- PyPI package processor implementation
- Rate limiting for API requests
- File filtering based on extensions
- Configuration via environment variables
- Structured JSON output format

### Changed
- Improved GitHub API integration
- Enhanced error handling and reporting
- Better metadata extraction from repositories

### Fixed
- Archive extraction for zip and tar formats
- URL parsing and validation issues
- Output directory handling

## [0.5.0] - 2023-01-20

### Added
- GitHub repository processor implementation
- Caching system for repository downloads
- Command-line argument parsing
- Basic configuration management
- Integration with GitHub API

### Changed
- Modular architecture with processor interfaces
- Improved error handling
- Enhanced documentation

### Fixed
- Output directory creation issues
- HTTP request error handling
- Path handling for cross-platform compatibility

## [0.1.0] - 2023-01-05

### Added
- Initial project structure and architecture
- Core interfaces and data models
- Basic error handling framework
- Documentation and README
- License and contribution guidelines 