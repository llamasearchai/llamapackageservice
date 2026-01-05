# LlamaPackageService - Python Implementation

A comprehensive package processing and analysis tool - Python port of the Rust implementation.

## Features

- Process GitHub repositories and organizations
- Process PyPI packages
- Process NPM packages
- Process Rust crates
- Process Go packages
- Process local files and directories
- Analytics and metrics collection
- Caching for improved performance
- AI-powered code analysis with OpenAI integration
- REST API for programmatic access

## Installation

```bash
# Install from source
pip install -e ./python

# Or install with development dependencies
pip install -e "./python[dev]"
```

## Usage

### Command Line Interface

```bash
# Interactive mode
llamapackage

# Process a URL directly
llamapackage --url https://github.com/username/repository

# Specify output directory
llamapackage --url https://pypi.org/project/requests --output ./my-output

# Generate index file after processing
llamapackage --url https://github.com/user/repo --index

# Legacy UI mode
llamapackage --legacy-ui
```

### Supported URL Types

- **Local Files/Directories**: `./my-project`, `/path/to/code`
- **GitHub**: `https://github.com/username/repository`
- **PyPI**: `https://pypi.org/project/package-name` or `pip install package-name`
- **NPM**: `https://www.npmjs.com/package/package-name` or `npm install package-name`
- **Rust Crates**: `https://crates.io/crates/crate-name`
- **Go Packages**: `https://pkg.go.dev/github.com/username/package`

### Python API

```python
from llamapackageservice import Config, ProcessorFactory
import asyncio

async def main():
    config = Config(output_dir="./output")
    processor = ProcessorFactory.create_processor("https://github.com/rust-lang/rust")
    await processor.process("https://github.com/rust-lang/rust", config.output_dir, config)

asyncio.run(main())
```

### REST API

Start the API server:

```bash
llamapackage --api
```

Then use the API:

```bash
# Submit a processing job
curl -X POST http://localhost:8000/api/process \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com/user/repo"}'

# Check job status
curl http://localhost:8000/api/status/{job_id}

# Health check
curl http://localhost:8000/health
```

## Configuration

Configuration can be done via environment variables or a config file:

```bash
# Environment variables
export GITHUB_TOKEN=your_github_token
export OPENAI_API_KEY=your_openai_key
export OUTPUT_DIR=./output
```

Or create a config file at `~/.config/llama-package-service/config.toml`:

```toml
[general]
output_dir = "./output"

[api_keys]
github_token = "your_token"
openai_api_key = "your_key"

[processing]
max_concurrent_downloads = 5
max_concurrent_analyses = 3

[rate_limits]
github_api = 5000
pypi_api = 100
npm_api = 100
```

## Project Structure

```
python/
├── src/
│   └── llamapackageservice/
│       ├── __init__.py          # Package exports
│       ├── cli.py               # CLI interface
│       ├── config.py            # Configuration management
│       ├── error.py             # Error types
│       ├── cache.py             # Caching mechanisms
│       ├── api.py               # REST API
│       ├── parallel.py          # Parallel processing
│       ├── output_organizer.py  # Output file organization
│       ├── processors/          # Package processors
│       │   ├── __init__.py
│       │   ├── base.py          # Base processor interface
│       │   ├── factory.py       # Processor factory
│       │   ├── github.py        # GitHub processor
│       │   ├── pypi.py          # PyPI processor
│       │   ├── npm.py           # NPM processor
│       │   ├── crates.py        # Rust crates processor
│       │   ├── go.py            # Go packages processor
│       │   └── local.py         # Local file processor
│       ├── agents/              # AI agents
│       │   ├── __init__.py
│       │   ├── openai_agent.py
│       │   └── analysis.py
│       └── utils/               # Utilities
│           ├── __init__.py
│           ├── path.py
│           └── retry.py
└── tests/                       # Test suite
```

## Development

```bash
# Install development dependencies
pip install -e "./python[dev]"

# Run tests
pytest

# Run with coverage
pytest --cov=llamapackageservice

# Format code
black src tests

# Lint code
ruff check src tests

# Type check
mypy src
```

## License

MIT License - see LICENSE file for details.
