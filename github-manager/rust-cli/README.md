# LlamaSearchAI Repository Manager - Rust CLI

A powerful command-line interface with an interactive terminal browser for managing the LlamaSearchAI GitHub organization.

## Features

### 🚀 Core Features
- **Terminal UI Browser**: Navigate repositories with an intuitive TUI
- **Multi-View System**: Repository list, details, file explorer, code viewer, and more
- **Syntax Highlighting**: View code with full syntax highlighting support
- **Advanced Search**: Search across all repositories with regex support
- **Security Scanning**: Built-in vulnerability detection and secret scanning
- **Analytics Dashboard**: Monitor repository health and metrics
- **Workflow Management**: Execute and monitor GitHub Actions
- **Real-time Updates**: WebSocket support for live data

### 🎨 Terminal Browser Views
1. **Repository List** (Alt+1): Browse and filter all repositories
2. **Repository Detail** (Alt+2): Comprehensive repository information
3. **File Explorer** (Alt+3): Navigate repository file structure
4. **Code View** (Alt+4): Syntax-highlighted code viewer
5. **Search** (Alt+5): Cross-repository search
6. **Analytics** (Alt+6): Metrics and insights
7. **Workflows** (Alt+7): GitHub Actions management
8. **Security** (Alt+8): Security scanning and monitoring
9. **Help** (Alt+9): Interactive help system

## Installation

### Prerequisites
- Rust 1.70+ and Cargo
- Git
- GitHub personal access token (optional but recommended)

### Building from Source
```bash
cd rust-cli
cargo build --release
```

### Installing
```bash
cargo install --path .
```

## Configuration

Create a configuration file at `~/.llamasearch/config.toml`:

```toml
[github]
organization = "llamasearchai"
token = "ghp_your_token_here"  # Or use GITHUB_TOKEN env var

[ui]
theme = "dark"
syntax_theme = "base16-ocean.dark"
refresh_rate_ms = 250

[features]
auto_sync = true
real_time_updates = true
ai_analysis = true
```

## Usage

### Terminal UI Mode
```bash
# Launch the interactive terminal browser
llamasearch tui

# With custom config
llamasearch --config ~/my-config.toml tui
```

### Command Line Mode
```bash
# Scan repositories
llamasearch scan --org llamasearchai

# Clone repositories
llamasearch clone repo1 repo2 --parallel 4

# Search across repos
llamasearch search "TODO" --regex -C 5

# Generate concatenated files
llamasearch generate --all --format markdown

# Run security scan
llamasearch security scan llamaagent --deep

# Analyze repository
llamasearch analyze llamagraph --analysis-type all
```

## Keyboard Shortcuts

### Global
- `Ctrl+Q`: Quit application
- `Ctrl+H`: Show help
- `Ctrl+S`: Global search
- `Ctrl+R`: Refresh current view
- `Alt+1-9`: Switch views by number
- `Esc`: Go back / Cancel

### Navigation
- `j/↓`: Move down
- `k/↑`: Move up
- `g`: Go to top
- `G`: Go to bottom
- `Enter`: Select/Open
- `/`: Filter/Search
- `Tab`: Next element
- `Shift+Tab`: Previous element

### View-Specific

#### Repository List
- `s/S`: Sort by stars
- `n/N`: Sort by name
- `u/U`: Sort by update time
- `l/L`: Sort by language

#### File Explorer
- `Space`: Toggle directory
- `h/←`: Collapse or go to parent
- `l/→`: Expand directory

#### Code View
- `0`: Beginning of line
- `$`: End of line
- `PageUp/Down`: Scroll by page
- `/`: Search in file
- `n/N`: Next/Previous match

## Development

### Project Structure
```
rust-cli/
├── src/
│   ├── browser/          # Terminal UI components
│   │   ├── views/        # Individual view implementations
│   │   ├── state.rs      # Application state management
│   │   └── mod.rs        # Browser orchestration
│   ├── cli.rs            # Command-line interface
│   ├── config.rs         # Configuration management
│   ├── error.rs          # Error handling
│   ├── github.rs         # GitHub API client
│   └── main.rs           # Entry point
├── tests/                # Test suite
└── Cargo.toml           # Dependencies
```

### Running Tests
```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run lints
cargo clippy -- -D warnings

# Check formatting
cargo fmt -- --check
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and lints
5. Submit a pull request

## Performance

- **Parallel Operations**: Configurable parallelism for network operations
- **Smart Caching**: Adaptive caching with TTL management
- **Lazy Loading**: Large files are loaded on-demand
- **Streaming**: Support for large file operations

## Security

- **Token Storage**: Secure token storage with encryption
- **RBAC**: Role-based access control
- **Audit Logging**: All operations are logged
- **Vulnerability Scanning**: Built-in security checks

## Troubleshooting

### Common Issues

1. **GitHub API Rate Limiting**
   - Use a personal access token
   - Enable caching in config

2. **Terminal Display Issues**
   - Ensure terminal supports Unicode
   - Try different terminal emulators

3. **Performance Issues**
   - Reduce `max_parallel_operations`
   - Increase cache size
   - Enable compression

## License

MIT License - see LICENSE file for details

## Credits

Built with:
- [Ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [Octocrab](https://github.com/XAMPPRocky/octocrab) - GitHub API
- [Syntect](https://github.com/trishume/syntect) - Syntax highlighting

---

Made with ❤️ for the LlamaSearchAI ecosystem