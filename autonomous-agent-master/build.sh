#!/bin/bash
set -e

echo "🚀 Building Autonomous Agent Master System"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Functions
print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check dependencies
print_step "Checking dependencies..."

# Check Rust
if ! command -v cargo &> /dev/null; then
    print_error "Rust not installed. Please install from https://rustup.rs/"
    exit 1
fi

# Check Python
if ! command -v python3 &> /dev/null; then
    print_error "Python 3 not installed"
    exit 1
fi

# Check PostgreSQL
if ! command -v psql &> /dev/null; then
    print_warning "PostgreSQL client not found. Database features may be limited"
fi

# Check Docker (optional)
if command -v docker &> /dev/null; then
    print_success "Docker found - container deployment available"
else
    print_warning "Docker not found - container deployment unavailable"
fi

print_success "Dependencies checked"

# Create necessary directories
print_step "Creating directories..."
mkdir -p {logs,data,models,cache,temp}
print_success "Directories created"

# Build the system
print_step "Building Rust components..."

# Update dependencies
cargo update

# Build in release mode with optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release

print_success "Rust build completed"

# Set up Python environment
print_step "Setting up Python environment..."

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    python3 -m venv venv
fi

# Activate virtual environment
source venv/bin/activate

# Install Python dependencies
pip install --upgrade pip
pip install -r requirements.txt 2>/dev/null || true

print_success "Python environment ready"

# Set up database
print_step "Setting up database..."

if command -v psql &> /dev/null; then
    # Create database if it doesn't exist
    createdb agent_master 2>/dev/null || print_warning "Database already exists"
    
    # Run migrations
    if [ -f "migrations/schema.sql" ]; then
        psql -d agent_master -f migrations/schema.sql
        print_success "Database migrations applied"
    fi
else
    print_warning "Skipping database setup - PostgreSQL not available"
fi

# Download ML models
print_step "Downloading ML models..."

# Create models directory
mkdir -p models

# Download pre-trained models (example)
if [ ! -f "models/decision_model.onnx" ]; then
    print_warning "ML models not found. The system will train new models on first run"
fi

# Generate configuration
print_step "Generating configuration..."

# Copy default config if custom doesn't exist
if [ ! -f "config/master.toml" ]; then
    cp config/master.toml.example config/master.toml 2>/dev/null || true
fi

# Validate configuration
./target/release/agent-master validate-config || print_warning "Configuration validation failed"

# Create systemd service
print_step "Creating systemd service..."

cat > agent-master.service << EOF
[Unit]
Description=Autonomous Agent Master System
After=network.target postgresql.service

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/target/release/agent-master start
Restart=always
RestartSec=10
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Security
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

print_success "Service file created"

# Create Docker image (if Docker available)
if command -v docker &> /dev/null; then
    print_step "Building Docker image..."
    
    cat > Dockerfile << 'EOF'
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/agent-master /usr/local/bin/
COPY --from=builder /app/config /etc/agent-master/

EXPOSE 8080 9090

CMD ["agent-master", "start"]
EOF

    docker build -t agent-master:latest . || print_warning "Docker build failed"
    print_success "Docker image built"
fi

# Create run script
print_step "Creating run scripts..."

cat > run.sh << 'EOF'
#!/bin/bash
# Autonomous Agent Master Run Script

# Load environment
if [ -f .env ]; then
    export $(cat .env | xargs)
fi

# Check if already running
if [ -f "agent-master.pid" ]; then
    PID=$(cat agent-master.pid)
    if ps -p $PID > /dev/null; then
        echo "Agent Master is already running (PID: $PID)"
        exit 1
    fi
fi

# Start the system
echo "Starting Autonomous Agent Master..."
./target/release/agent-master start --daemon --pid-file agent-master.pid
EOF

chmod +x run.sh

cat > stop.sh << 'EOF'
#!/bin/bash
# Stop script

if [ -f "agent-master.pid" ]; then
    PID=$(cat agent-master.pid)
    echo "Stopping Agent Master (PID: $PID)..."
    kill -TERM $PID
    rm -f agent-master.pid
else
    echo "Agent Master is not running"
fi
EOF

chmod +x stop.sh

print_success "Run scripts created"

# Create monitoring dashboard
print_step "Setting up monitoring..."

cat > docker-compose.monitoring.yml << 'EOF'
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  prometheus_data:
  grafana_data:
EOF

print_success "Monitoring configuration created"

# Final summary
echo
echo -e "${GREEN}🎉 Build completed successfully!${NC}"
echo
echo "Next steps:"
echo "1. Configure environment variables in .env file"
echo "2. Update config/master.toml with your settings"
echo "3. Start the system:"
echo "   ./run.sh                    # Run in background"
echo "   ./target/release/agent-master start  # Run in foreground"
echo
echo "Additional commands:"
echo "   agent-master health         # Check system health"
echo "   agent-master agent list     # List active agents"
echo "   agent-master task submit    # Submit a task"
echo "   agent-master diagnose       # Run diagnostics"
echo
echo "Monitoring:"
echo "   docker-compose -f docker-compose.monitoring.yml up -d"
echo "   Open http://localhost:3000 for Grafana dashboard"
echo
echo "Documentation: docs/README.md"
echo "Support: https://github.com/your-org/agent-master"
echo

# Create environment template
cat > .env.example << 'EOF'
# Autonomous Agent Master Environment Configuration

# API Keys
GITHUB_TOKEN=your_github_token
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key

# Database
DATABASE_URL=postgresql://agent:password@localhost/agent_master

# Monitoring
PROMETHEUS_URL=http://localhost:9090
GRAFANA_URL=http://localhost:3000

# Notifications
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
WEBHOOK_TOKEN=your_webhook_token

# Resource Limits
MAX_CPU_PERCENT=80
MAX_MEMORY_GB=32

# Feature Flags
ENABLE_AUTONOMOUS_DEPLOYMENT=true
ENABLE_SELF_HEALING=true
EOF

print_success "Environment template created (.env.example)"
print_warning "Don't forget to create .env file with your actual values!"