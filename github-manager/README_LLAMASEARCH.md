# 🦙 LlamaSearchAI GitHub Organization Manager

A powerful, AI-enhanced management system specifically designed for the LlamaSearchAI GitHub organization. This tool provides complete control over all repositories, with capabilities for generating concatenated text files, analyzing codebases, and managing the entire ecosystem.

## 🚀 Key Features

### Repository Management
- **Complete Organization Control**: Manage all 199+ LlamaSearchAI repositories
- **Bulk Operations**: Clone, update, analyze all repos with single commands
- **Smart Concatenation**: Generate organized text files from all repositories
- **AI-Powered Analysis**: Deep insights using Ollama integration

### Specialized for LlamaSearchAI
- **Ecosystem Understanding**: Knows the relationships between llamaagent, llamagraph, llama-cli, etc.
- **Language-Aware**: Handles Python, Rust, JavaScript, and documentation
- **Project Templates**: Create new projects following LlamaSearchAI standards
- **Dependency Tracking**: Monitor dependencies across the ecosystem

### Advanced Capabilities
- **Natural Language Commands**: "Find all TODO comments in Python files"
- **Real-time Monitoring**: Watch for commits, issues, and PRs
- **Documentation Generation**: Auto-generate comprehensive docs
- **Web Dashboard**: Visual management interface

## 📋 Prerequisites

- macOS (optimized for Mac)
- Python 3.11+
- Git
- GitHub personal access token with repo access
- Ollama (for AI features)

## 🛠️ Installation

### Quick Setup

```bash
# Clone the repository
cd github-manager

# Run LlamaSearchAI-specific setup
./setup-llamasearch.sh

# Set your GitHub token
export GITHUB_TOKEN='your_github_personal_access_token'
```

### Create GitHub Token

1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Select scopes:
   - `repo` (full control)
   - `read:org` (read org data)
   - `workflow` (update workflows)
4. Copy token and export it

## 🎯 Usage

### Basic Commands

```bash
# Scan all LlamaSearchAI repositories
./llamasearch-manager scan

# Clone/update all repositories
./llamasearch-manager clone

# Generate concatenated text files
./llamasearch-manager generate

# Analyze the ecosystem
./llamasearch-manager analyze

# Create comprehensive documentation
./llamasearch-manager docs
```

### Quick Scripts

```bash
# Run complete workflow (scan, clone, generate, analyze)
./llama-full-run.sh

# Daily sync routine
./llama-daily-sync.sh

# Quick repository scan
./llama-scan.sh

# Generate all concatenated files
./llama-generate.sh
```

### Advanced Usage

#### Generate Concatenated Files

```bash
# Generate for all repos
./llamasearch-manager generate

# Generate for specific repos
./llamasearch-manager generate -r llamaagent -r llamagraph

# Custom output directory
./llamasearch-manager generate -o ~/my-output
```

Output structure:
```
output/llamasearchai/20240122_143022/
├── llamaagent_concatenated.txt
├── llamagraph_concatenated.txt
├── llama-cli_concatenated.txt
├── ...
├── llamasearchai_all_repos_master.txt
├── index.yaml
└── AI_SUMMARY.md
```

#### Search Across Repositories

```bash
# Natural language search
./llamasearch-manager search -q "implementation of SPRE framework"

# Search in specific repo
./llamasearch-manager search -q "async functions" -r llamaagent
```

#### Create New Project

```bash
# Create with LlamaSearchAI template
./llamasearch-manager create my-llama-service

# Use specific template
./llamasearch-manager create my-agent -t llamaagent
```

#### Update Repository

```bash
# Update description and topics
./llamasearch-manager update llamaagent \
  -d "Production-ready autonomous agent framework" \
  -t ai -t agents -t llm
```

### Web Dashboard

```bash
# Start web dashboard
./llamasearch-manager dashboard

# Access at http://localhost:8000
```

Features:
- Visual repository overview
- Real-time monitoring
- AI command interface
- Batch operations

### Interactive Examples

```bash
# Run interactive examples
python examples/llamasearch_example.py
```

## 📁 Output Structure

### Concatenated Files

Each repository gets a concatenated text file containing:
- Repository metadata (stars, description, etc.)
- README and documentation
- All source code files
- Configuration files
- File listing and statistics

### Master File

`llamasearchai_all_repos_master.txt` contains:
- Table of contents
- All individual repository files
- Complete codebase in one file

### Index and Summary

- `index.yaml`: Machine-readable index
- `AI_SUMMARY.md`: AI-generated analysis

## 🏗️ Architecture

The manager understands the LlamaSearchAI ecosystem:

```
LlamaSearchAI Ecosystem
├── Core Framework
│   └── llamaagent (Production agent framework)
├── Knowledge Tools
│   └── llamagraph (Entity extraction & graphs)
├── Interfaces
│   └── llama-cli (Unified CLI)
├── Extensions
│   ├── OpenPersona (Multi-agent personalities)
│   └── llama-metasearch (Meta-search aggregation)
└── Infrastructure
    └── Various supporting repositories
```

## 🔧 Configuration

Edit `config/llamasearch_config.yaml`:

```yaml
organization: "llamasearchai"
local_repos_path: "~/Development/llamasearchai"

# Repository-specific settings
repository_settings:
  llamaagent:
    priority: high
    auto_sync: true
    generate_docs: true

# Concatenation rules
concatenation:
  max_file_size: 1048576  # 1MB
  include_patterns:
    python: ["*.py", "requirements.txt"]
    rust: ["*.rs", "Cargo.toml"]
```

## 🤖 AI Features

### Powered by Ollama

- **Code Analysis**: Understand repository structure and purpose
- **Dependency Analysis**: Track dependencies across ecosystem
- **Architecture Insights**: How components work together
- **Natural Language**: Convert queries to code searches

### Example AI Commands

```bash
# Analyze specific repository
./llamasearch-manager analyze

# Natural language search
./llamasearch-manager search -q "find all async function implementations"

# In web dashboard
"Show me all repositories that use FastAPI"
"Analyze the security of llamaagent"
```

## 📊 Monitoring

### Watch for Changes

```bash
# Monitor key repositories
./llamasearch-manager watch
```

Monitors:
- New commits
- Issues created
- Pull requests
- Releases

### MCP Server Integration

Start MCP servers for advanced features:

```bash
# In separate terminal
./start-mcp-servers.sh
```

## 🚀 Workflow Examples

### Daily Development Workflow

```bash
# Morning sync
./llama-daily-sync.sh

# Work on specific repo
cd ~/Development/llamasearchai/llamaagent
# Make changes...

# Generate fresh docs
./llamasearch-manager generate -r llamaagent

# Analyze changes
./llamasearch-manager analyze
```

### Research Workflow

```bash
# Generate complete concatenation
./llama-full-run.sh

# Output will be in:
# ~/Documents/llamasearchai_exports/

# Use concatenated files for:
# - LLM analysis
# - Code review
# - Documentation
# - Training data
```

### Project Creation Workflow

```bash
# Create new service
./llamasearch-manager create llama-new-service

# Follow LlamaSearchAI patterns
cd ~/Development/llamasearchai/llama-new-service
source venv/bin/activate
pip install -r requirements.txt
```

## 📈 Best Practices

1. **Regular Syncs**: Run daily sync to keep repos updated
2. **Concatenation Strategy**: Generate files before major analysis
3. **Use AI Features**: Leverage natural language for complex searches
4. **Monitor Changes**: Use watch mode for active development
5. **Documentation**: Generate docs after significant changes

## 🐛 Troubleshooting

### GitHub Token Issues
```bash
# Verify token
echo $GITHUB_TOKEN

# Test token
curl -H "Authorization: token $GITHUB_TOKEN" https://api.github.com/user
```

### Ollama Not Responding
```bash
# Start Ollama
ollama serve

# Pull required model
ollama pull llama3.1:8b
```

### Permission Errors
```bash
# Fix permissions
chmod +x llamasearch-manager
chmod +x *.sh
```

## 🔗 Related Documentation

- [Architecture Overview](docs/architecture.md)
- [Main README](README.md)
- [Examples](examples/llamasearch_example.py)

## 📄 License

MIT License - Matches LlamaSearchAI projects

## 🤝 Contributing

1. Follow LlamaSearchAI coding standards
2. Test with multiple repositories
3. Update documentation
4. Submit PR with clear description

---

Built specifically for managing the LlamaSearchAI ecosystem 🦙