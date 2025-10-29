#!/bin/bash
set -e

echo "🚀 Building Llama-Operate Integration System"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check prerequisites
print_step "Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install Rust."
    exit 1
fi

if ! command -v python3 &> /dev/null; then
    print_error "Python 3 not found. Please install Python 3.9+."
    exit 1
fi

print_success "Prerequisites checked"

# Build LlamaAgents
print_step "Building LlamaAgents..."
cd llamaagent
cargo build --release
print_success "LlamaAgents built"

# Set up Python environment for Operate Enhanced
print_step "Setting up Operate Enhanced..."
cd ../operate-enhanced

if [ ! -d "venv" ]; then
    python3 -m venv venv
fi

source venv/bin/activate
pip install -e .
print_success "Operate Enhanced set up"

# Build integration system
print_step "Building Llama-Operate Integration..."
cd ../llama-operate-integration

# Run database migrations
print_step "Setting up database..."
if [ ! -f "llama-operate.db" ]; then
    sqlite3 llama-operate.db < migrations/001_initial.sql
    print_success "Database created"
else
    print_success "Database already exists"
fi

# Build the integration
cargo build --release
print_success "Integration system built"

# Run tests
print_step "Running tests..."
cargo test --release
print_success "Tests passed"

# Create run script
print_step "Creating run script..."
cat > run.sh << 'EOF'
#!/bin/bash
# Activate Python environment
source ../operate-enhanced/venv/bin/activate

# Export required environment variables
export RUST_LOG=${RUST_LOG:-info}
export PYTHONPATH="../operate-enhanced:$PYTHONPATH"

# Check for required environment variables
if [ -z "$GITHUB_TOKEN" ]; then
    echo "Warning: GITHUB_TOKEN not set"
fi

if [ -z "$OPENAI_API_KEY" ]; then
    echo "Warning: OPENAI_API_KEY not set"
fi

# Run the system
exec ./target/release/llama-operate "$@"
EOF

chmod +x run.sh
print_success "Run script created"

# Create systemd service file (optional)
print_step "Creating systemd service file..."
cat > llama-operate.service << EOF
[Unit]
Description=Llama-Operate Integration System
After=network.target

[Service]
Type=simple
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/run.sh start
Restart=always
RestartSec=10
Environment="RUST_LOG=info"
Environment="GITHUB_TOKEN=${GITHUB_TOKEN}"
Environment="OPENAI_API_KEY=${OPENAI_API_KEY}"

[Install]
WantedBy=multi-user.target
EOF
print_success "Service file created"

# Summary
echo
echo -e "${GREEN}🎉 Build completed successfully!${NC}"
echo
echo "To run the system:"
echo "  ./run.sh start"
echo
echo "To run specific commands:"
echo "  ./run.sh repo add owner/repo"
echo "  ./run.sh workflow trigger code_review --repo owner/repo"
echo "  ./run.sh monitor --status"
echo
echo "To install as a service:"
echo "  sudo cp llama-operate.service /etc/systemd/system/"
echo "  sudo systemctl enable llama-operate"
echo "  sudo systemctl start llama-operate"
echo
echo "Make sure to set these environment variables:"
echo "  export GITHUB_TOKEN=your-github-token"
echo "  export OPENAI_API_KEY=your-openai-key"
echo