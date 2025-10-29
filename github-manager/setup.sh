#!/bin/bash

echo "======================================"
echo "GitHub Repository Manager Setup"
echo "======================================"

# Check for required tools
check_dependency() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 is not installed"
        return 1
    else
        echo "✓ $1 is installed"
        return 0
    fi
}

echo -e "\n📋 Checking dependencies..."

# Check Python
if ! check_dependency python3; then
    echo "Installing Python via Homebrew..."
    brew install python@3.11
fi

# Check Git
if ! check_dependency git; then
    echo "Installing Git via Homebrew..."
    brew install git
fi

# Check Ollama
if ! check_dependency ollama; then
    echo "Installing Ollama..."
    curl -fsSL https://ollama.ai/install.sh | sh
fi

# Install Python dependencies
echo -e "\n📦 Installing Python dependencies..."
pip3 install -r requirements.txt

# Pull Ollama models
echo -e "\n🤖 Pulling Ollama models..."
ollama pull llama3.1:8b
ollama pull llava:13b

# Install llm CLI tools
echo -e "\n🔧 Installing LLM CLI tools..."
pip3 install llm llm-ollama

# Setup configuration
echo -e "\n⚙️  Setting up configuration..."
mkdir -p config

if [ ! -f config/github_config.yaml ]; then
    echo "Creating default configuration..."
    cat > config/github_config.yaml << EOF
github_token: "${GITHUB_TOKEN}"
local_repos_path: "~/Development"
backup_path: "~/Backups/repos"
mcp_servers:
  - name: github
    host: localhost
    port: 3001
    capabilities:
      - repository_management
      - issue_tracking
      - pull_requests
  - name: filesystem
    host: localhost
    port: 3002
    capabilities:
      - file_operations
      - directory_scanning
  - name: project
    host: localhost
    port: 3003
    capabilities:
      - project_analysis
      - dependency_management
ollama_host: "http://localhost:11434"
log_level: INFO
web_dashboard:
  host: "0.0.0.0"
  port: 8000
EOF
fi

# Create directories
echo -e "\n📁 Creating directories..."
mkdir -p logs
mkdir -p ~/.github-manager/cache
mkdir -p ~/Development
mkdir -p ~/Backups/repos

# Create static directory for web dashboard
mkdir -p src/web/static

# Check GitHub token
if [ -z "$GITHUB_TOKEN" ]; then
    echo -e "\n⚠️  Warning: GITHUB_TOKEN environment variable is not set"
    echo "Please set it with: export GITHUB_TOKEN='your_token_here'"
fi

# Create launcher script
echo -e "\n🚀 Creating launcher scripts..."
cat > github-manager << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
python3 -m src.cli.commands "$@"
EOF
chmod +x github-manager

# Create MCP server launcher
cat > start-mcp-servers.sh << 'EOF'
#!/bin/bash
echo "Starting MCP servers..."

# Start GitHub MCP server
python3 -m src.mcp.github_server &
GITHUB_PID=$!
echo "GitHub MCP server started (PID: $GITHUB_PID)"

# Trap to clean up on exit
trap "kill $GITHUB_PID 2>/dev/null" EXIT

echo "MCP servers are running. Press Ctrl+C to stop."
wait
EOF
chmod +x start-mcp-servers.sh

# Create web dashboard launcher
cat > start-dashboard.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
echo "Starting GitHub Repository Manager Dashboard..."
python3 -m src.cli.commands dashboard
EOF
chmod +x start-dashboard.sh

echo -e "\n✅ Setup complete!"
echo -e "\nUsage:"
echo "  • CLI: ./github-manager --help"
echo "  • Dashboard: ./start-dashboard.sh"
echo "  • MCP Servers: ./start-mcp-servers.sh"
echo -e "\nQuick start:"
echo "  1. Export your GitHub token: export GITHUB_TOKEN='your_token'"
echo "  2. Start MCP servers: ./start-mcp-servers.sh (in a separate terminal)"
echo "  3. Run the CLI: ./github-manager list-repos"
echo "  4. Or start the dashboard: ./start-dashboard.sh"