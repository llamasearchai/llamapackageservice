# Operate Enhanced - AI-Powered Computer Automation Framework

An advanced self-operating computer framework that enables AI models to control computers, now with GitHub integration, security features, and performance optimizations.

## 🚀 Features

### Core Capabilities
- **Multi-Model Support**: OpenAI GPT-4o, Anthropic Claude 3, Google Gemini, and local models
- **Advanced Operation Orchestration**: Sequential and parallel execution with dependency resolution
- **State Management**: Checkpoints, transactions, and pattern learning
- **Security Framework**: Sandboxing, permission system, and audit logging
- **Performance Optimization**: Intelligent caching, parallel execution, and screenshot optimization

### GitHub Integration
- **Repository Management**: Clone, analyze, and monitor repositories
- **Pull Request Automation**: Create and review PRs with AI assistance
- **Code Analysis**: Language detection, dependency analysis, and structure mapping
- **CI/CD Integration**: Trigger workflows and monitor builds

### Enterprise Features
- **Web Dashboard**: Real-time monitoring and control
- **REST API**: Full programmatic access
- **WebSocket Support**: Live updates and remote control
- **Audit Logging**: Complete operation history
- **Pattern Learning**: Reusable automation patterns

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/operate-enhanced.git
cd operate-enhanced

# Install with poetry
poetry install

# Or with pip
pip install -e .
```

## 🛠️ Configuration

### Environment Variables
```bash
# AI Model API Keys
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"
export GOOGLE_API_KEY="your-key"

# GitHub Integration
export GITHUB_TOKEN="your-token"
```

### Configuration File
```yaml
# config/operate.yaml
operate:
  models:
    primary: gpt-4o
    fallback: [claude-3, gemini-pro]
  
  security:
    sandbox_default: true
    require_confirmation:
      - system_commands
      - file_deletion
  
  performance:
    cache_enabled: true
    max_workers: 4
```

## 🚀 Quick Start

### Command Line Interface

```bash
# Basic operation
operate run "Open browser and search for Python tutorials"

# With specific model
operate run "Create a new document" --model claude-3

# GitHub operations
operate github "owner/repo" --pr-number 123

# Run saved pattern
operate pattern "daily-workflow"

# Start web dashboard
operate server --port 8000
```

### Python API

```python
from operate import OperationOrchestrator, Action, ActionType
from operate.integrations import GitHubManager

# Initialize
orchestrator = OperationOrchestrator()
github = GitHubManager(token="your-token")

# Execute operation
action = Action(
    type=ActionType.CLICK,
    target="Submit Button"
)
result = await orchestrator.execute_operation(action)

# GitHub integration
analysis = await github.analyze_codebase("./my-project")
pr = await github.create_pull_request(
    "owner/repo",
    title="Feature: Add new capability",
    body="This PR adds...",
    head_branch="feature-branch"
)
```

## 🔒 Security

### Sandbox Mode
By default, operations run in sandbox mode with restrictions:
- No system commands
- No file deletions
- Limited network access

### Permission System
```python
# Grant specific permissions
security_guardian.grant_permission("file_write")

# Create custom security rules
rule = SecurityRule(
    type=RuleType.BLACKLIST,
    pattern="rm -rf",
    level=SecurityLevel.CRITICAL
)
security_guardian.add_rule(rule)
```

## 📊 Performance

### Caching System
- Screenshot caching with change detection
- Model response caching
- OCR result caching

### Parallel Execution
```python
# Execute multiple operations in parallel
operations = [
    Operation(action=click_action1),
    Operation(action=click_action2),
    Operation(action=type_action)
]
results = await orchestrator.execute_parallel(operations)
```

## 🌐 Web Dashboard

Access the dashboard at `http://localhost:8000` after starting the server:

- Real-time screen mirroring
- Operation history visualization
- Performance metrics
- Security controls
- Pattern management

## 🧪 Testing

```bash
# Run tests
pytest tests/

# With coverage
pytest --cov=operate tests/

# Integration tests
pytest tests/integration/ --github-token=$GITHUB_TOKEN
```

## 📚 Advanced Usage

### Custom Plugins

```python
from operate.interfaces import IPlugin, Action, OperationResult

class CustomPlugin(IPlugin):
    def get_name(self) -> str:
        return "my-plugin"
    
    async def execute(self, action: Action) -> OperationResult:
        # Custom implementation
        pass
```

### Pattern Creation

```python
# Save reusable patterns
actions = [
    Action(type=ActionType.CLICK, target="File"),
    Action(type=ActionType.CLICK, target="New Document"),
    Action(type=ActionType.TYPE, value="Hello World")
]
await state_manager.save_pattern("create-document", actions)
```

### Monitoring

```python
# Set up repository monitoring
await github.monitor_repository(
    "owner/repo",
    callbacks={
        "on_commit": handle_new_commit,
        "on_pr": handle_new_pr,
        "on_issue": handle_new_issue
    }
)
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

MIT License - see LICENSE file for details

## 🙏 Acknowledgments

Built upon the original self-operating-computer framework with significant enhancements for production use.