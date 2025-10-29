# GitHub Repository Manager

A comprehensive GitHub repository management system with AI-powered analysis, MCP server integration, and natural language command capabilities.

## Features

### 🚀 Core Capabilities
- **Repository Management**: Scan, sync, and manage local/remote repositories
- **AI-Powered Analysis**: Use Ollama models for code analysis and insights
- **Natural Language Commands**: Execute complex operations using plain English
- **MCP Server Integration**: Seamless GitHub operations via Model Context Protocol
- **Visual Dashboard**: Web-based interface for repository management
- **Batch Operations**: Perform actions across multiple repositories

### 🤖 AI Features
- **Code Quality Analysis**: Automated assessment of repository structure
- **Visual Content Analysis**: Analyze diagrams and screenshots using vision models
- **Function Calling**: Advanced Ollama integration with tool execution
- **Smart Suggestions**: AI-driven recommendations for improvements

### 🔧 Technical Features
- **WebSocket Real-time Updates**: Live repository monitoring
- **Automated Dependency Updates**: Keep projects current
- **Issue Analysis**: AI-powered issue categorization and insights
- **Multi-language Support**: Works with any programming language

## Installation

### Prerequisites
- macOS (optimized for Mac)
- Python 3.11+
- Git
- GitHub personal access token

### Quick Setup

1. Clone and setup:
```bash
cd github-manager
chmod +x setup.sh
./setup.sh
```

2. Configure GitHub token:
```bash
export GITHUB_TOKEN='your_github_token_here'
```

3. Start MCP servers (in a separate terminal):
```bash
./start-mcp-servers.sh
```

4. Launch the dashboard:
```bash
./start-dashboard.sh
```

## Usage

### Command Line Interface

List repositories:
```bash
./github-manager list-repos
```

Analyze a repository:
```bash
./github-manager analyze /path/to/repo
```

Natural language commands:
```bash
./github-manager ai-command "find all Python files with TODO comments"
./github-manager ai-command "analyze security vulnerabilities in my code"
```

Create new repository:
```bash
./github-manager create-repo my-new-project --description "Amazing project" --private
```

Batch operations:
```bash
./github-manager batch sync repo1 repo2 repo3
./github-manager batch analyze repo1 repo2 repo3
```

### Web Dashboard

Access the dashboard at `http://localhost:8000` after running `./start-dashboard.sh`

Features:
- Visual repository overview
- Real-time sync status
- AI command interface
- Batch operation management
- Issue tracking and analysis

### Interactive Mode

Start interactive session:
```bash
./github-manager interactive
```

Commands in interactive mode:
- `list` - List all repositories
- `analyze <repo>` - Analyze repository
- `sync <repo>` - Sync with remote
- `ai <command>` - Execute AI command
- `help` - Show available commands
- `exit` - Exit interactive mode

## Configuration

Edit `config/github_config.yaml`:

```yaml
github_token: "your_token"
local_repos_path: "~/Development"
backup_path: "~/Backups/repos"
mcp_servers:
  - name: github
    host: localhost
    port: 3001
ollama_host: "http://localhost:11434"
log_level: INFO
```

## API Integration

### MCP Server Endpoints

The GitHub MCP server provides these tools:
- `list_repositories` - List all repositories
- `create_repository` - Create new repository
- `analyze_repository_issues` - Analyze issues
- `create_pull_request` - Create PR
- `review_pull_request` - AI-powered PR review
- `search_code` - Search across repositories
- `manage_secrets` - Handle repository secrets

### WebSocket Events

Connect to `ws://localhost:8000/ws` for real-time updates:

```javascript
{
  "type": "analyze_repo",
  "repo_path": "/path/to/repo"
}
```

## Architecture

```
github-manager/
├── src/
│   ├── core/           # Core business logic
│   ├── mcp/            # MCP server implementations
│   ├── cli/            # Command-line interface
│   ├── web/            # Web dashboard
│   └── utils/          # Utilities
├── config/             # Configuration files
├── scripts/            # Setup and utility scripts
└── tests/              # Test suite
```

## Advanced Usage

### Custom MCP Servers

Create custom MCP servers for specific workflows:

```python
from src.mcp.github_server import GitHubMCPServer

class CustomMCPServer(GitHubMCPServer):
    def __init__(self, token):
        super().__init__(token)
        self.tools['custom_tool'] = self.custom_tool
    
    async def custom_tool(self, **params):
        # Implementation
        return {"result": "success"}
```

### Ollama Function Extensions

Add custom tools for Ollama:

```python
from src.core.ollama_interface import OllamaInterface, Tool

ollama = OllamaInterface()
ollama.tools['custom_analysis'] = Tool(
    name='custom_analysis',
    description='Perform custom analysis',
    parameters={...},
    function=your_function
)
```

## Troubleshooting

### Common Issues

1. **Ollama not responding**:
   ```bash
   ollama serve  # Start Ollama service
   ```

2. **MCP connection failed**:
   - Ensure MCP servers are running
   - Check port availability

3. **GitHub API rate limits**:
   - Use authenticated requests
   - Implement caching

## Contributing

1. Fork the repository
2. Create feature branch
3. Commit changes
4. Push to branch
5. Create Pull Request

## License

MIT License - see LICENSE file for details

## Roadmap

- [ ] Docker containerization
- [ ] GitHub Actions integration
- [ ] Multi-user support
- [ ] Enhanced security scanning
- [ ] Plugin system
- [ ] Mobile app companion

## Support

- Documentation: [docs/](docs/)
- Issues: GitHub Issues
- Community: Discord/Slack

---

Built with ❤️ using Python, Ollama, and MCP