# Llama-Operate Integration System

A unified AI-powered development platform that combines LlamaAgents with Operate Enhanced for comprehensive GitHub repository and program management.

## 🚀 Features

### Core Capabilities
- **Unified Command Center**: Centralized management for all repositories and tasks
- **Intelligent Workflows**: Automated development workflows with AI assistance
- **Real-time Monitoring**: System health, metrics, and alerts
- **Self-healing Automation**: Automatic detection and remediation of issues
- **Python-Rust Bridge**: Seamless integration between Rust and Python components

### Key Components

#### 1. Command Center
- Repository registration and tracking
- Task creation and execution
- GitHub API integration
- Persistent state management

#### 2. Workflow Engine
- Pre-built workflows (code review, bug fix, optimization)
- Custom workflow creation
- Event-driven triggers
- Parallel step execution

#### 3. Monitoring System
- Real-time metrics collection
- Health checks
- Alert management
- Performance tracking

#### 4. Automation Engine
- Rule-based automation
- Self-healing capabilities
- Resource optimization
- Predictive maintenance

## 📦 Installation

### Prerequisites
- Rust 1.75+
- Python 3.9+
- GitHub token
- SQLite

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/llama-operate-integration.git
cd llama-operate-integration

# Install Rust dependencies
cargo build --release

# Set up Python environment
cd ../operate-enhanced
pip install -e .

# Configure environment
export GITHUB_TOKEN="your-github-token"
export OPENAI_API_KEY="your-openai-key"
```

## 🛠️ Configuration

Edit `config.toml`:

```toml
[python_config]
operate_path = "../operate-enhanced"
model = "gpt-4o"
sandbox_mode = true
cache_enabled = true

github_token = "${GITHUB_TOKEN}"
database_url = "sqlite://llama-operate.db"

[monitoring_config]
metrics_retention_hours = 168
alert_cooldown_minutes = 15
health_check_interval_seconds = 60
```

## 🚀 Quick Start

### Start the System

```bash
# Start all subsystems
cargo run -- --config config.toml start

# Start with debug logging
cargo run -- --config config.toml start --debug
```

### Repository Management

```bash
# Add a repository
cargo run -- repo add owner/repo

# Analyze repository
cargo run -- repo analyze owner/repo

# List repositories
cargo run -- repo list
```

### Workflow Management

```bash
# List available workflows
cargo run -- workflow list

# Trigger a workflow
cargo run -- workflow trigger code_review --repo owner/repo

# Check workflow status
cargo run -- workflow status <instance-id>
```

### Monitoring

```bash
# Show system status
cargo run -- monitor --status

# Show metrics
cargo run -- monitor --metrics
```

### Task Execution

```bash
# Run a specific task
cargo run -- task --type analyze_code --repo owner/repo
```

## 📚 Workflows

### Built-in Workflows

#### 1. Code Review
Automatically reviews pull requests and provides feedback.

```yaml
Triggers:
- pull_request.opened
- pull_request.synchronize

Steps:
1. Analyze changes
2. Check security, performance, style
3. Post review comments
```

#### 2. Bug Fix
Attempts to automatically fix reported bugs.

```yaml
Triggers:
- issues.labeled (bug)
- Manual

Steps:
1. Analyze issue
2. Reproduce bug
3. Generate fix
4. Test fix
5. Create PR
```

#### 3. Continuous Improvement
Regular code quality improvements.

```yaml
Triggers:
- Schedule (daily)

Steps:
1. Scan codebase
2. Identify improvements
3. Implement changes
4. Validate
5. Create PR
```

### Custom Workflows

Create custom workflows by implementing the `WorkflowDefinition`:

```rust
let workflow = WorkflowDefinition {
    id: "custom_workflow".to_string(),
    name: "My Custom Workflow".to_string(),
    triggers: vec![WorkflowTrigger::Manual],
    steps: vec![
        WorkflowStep {
            id: "step1".to_string(),
            step_type: StepType::Custom("my_step".to_string()),
            inputs: json!({}),
            on_success: Some("step2".to_string()),
            on_failure: None,
        },
    ],
};
```

## 🤖 Automation Rules

### Built-in Rules

#### Performance Optimization
```toml
[[automation_rules]]
id = "perf_optimization"
trigger.type = "metric"
trigger.name = "response_time"
trigger.condition = { type = "greater_than", value = 1000.0 }
action.type = "trigger_workflow"
action.workflow_id = "performance_optimization"
```

#### Memory Leak Detection
```toml
[[automation_rules]]
id = "memory_leak"
trigger.type = "metric"
trigger.name = "memory_usage"
trigger.condition = { type = "trend_increasing", threshold = 5.0 }
action.type = "composite"
actions = [
    { type = "create_alert", level = "warning" },
    { type = "trigger_workflow", workflow_id = "memory_analysis" }
]
```

## 📊 Monitoring

### Metrics
- System: CPU, memory, disk usage
- Application: Response time, error rate, throughput
- Custom: Any metric via `monitor.record_metric()`

### Health Checks
- GitHub API connectivity
- Python bridge status
- Database connection
- Custom health checks

### Alerts
- Automatic alert generation
- Cooldown periods
- Multiple severity levels
- Notification integration

## 🔧 Advanced Usage

### Python Bridge

```rust
// Execute Operate Enhanced operation
let operation = Operation {
    action_type: "click".to_string(),
    target: Some("Submit Button".to_string()),
    // ...
};

let result = bridge.execute_operation(operation).await?;
```

### Direct GitHub Operations

```rust
// Analyze repository
let analysis = bridge.github_operation(
    GitHubOperation::AnalyzeRepo { 
        repo_path: "./my-repo".to_string() 
    }
).await?;

// Create PR
let pr = bridge.github_operation(
    GitHubOperation::CreatePR {
        repo: "owner/repo".to_string(),
        title: "Feature".to_string(),
        // ...
    }
).await?;
```

### Custom Automation

```rust
// Register custom rule
automation.register_rule(Rule {
    id: "custom_rule".to_string(),
    trigger: RuleTrigger::Event {
        event_type: "custom_event".to_string(),
        filter: None,
    },
    action: RuleAction::Custom("my_action".to_string()),
    // ...
})?;
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_test

# Run with Python bridge
cargo test -- --ignored
```

## 📈 Performance

### Optimization Features
- Parallel task execution
- Intelligent caching
- Resource pooling
- Lazy loading

### Benchmarks
```bash
cargo bench
```

## 🔒 Security

- Sandboxed Python execution
- GitHub token encryption
- Audit logging
- Permission-based access

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests
4. Submit a pull request

## 📄 License

MIT License

## 🙏 Acknowledgments

Built on top of:
- LlamaAgents framework
- Operate Enhanced framework
- Rust ecosystem
- Python AI libraries