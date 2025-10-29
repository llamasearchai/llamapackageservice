#!/bin/bash

echo "======================================"
echo "🦙 LlamaSearchAI Manager Setup"
echo "======================================"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "llamasearch-manager" ]; then
    echo -e "${RED}Error: Please run this script from the github-manager directory${NC}"
    exit 1
fi

# Run the main setup if not already done
if [ ! -d "venv" ] && [ ! -f ".setup_complete" ]; then
    echo -e "\n${YELLOW}Running initial setup...${NC}"
    ./setup.sh
    touch .setup_complete
fi

# Create LlamaSearchAI specific directories
echo -e "\n${GREEN}Creating LlamaSearchAI directories...${NC}"
mkdir -p ~/Development/llamasearchai
mkdir -p ~/Documents/llamasearchai_exports
mkdir -p ~/.llamasearch/logs

# Check for GitHub token
if [ -z "$GITHUB_TOKEN" ]; then
    echo -e "\n${YELLOW}⚠️  GitHub token not found!${NC}"
    echo "Please set your GitHub token:"
    echo "  export GITHUB_TOKEN='your_personal_access_token'"
    echo ""
    echo "To create a token:"
    echo "1. Go to https://github.com/settings/tokens"
    echo "2. Click 'Generate new token (classic)'"
    echo "3. Select scopes: repo, workflow, read:org"
    echo "4. Copy the token and run the export command above"
    echo ""
    read -p "Press Enter to continue without token (limited functionality)..."
fi

# Create convenience scripts
echo -e "\n${GREEN}Creating convenience scripts...${NC}"

# Quick scan script
cat > llama-scan.sh << 'EOF'
#!/bin/bash
./llamasearch-manager scan
EOF
chmod +x llama-scan.sh

# Quick generate script
cat > llama-generate.sh << 'EOF'
#!/bin/bash
echo "🦙 Generating concatenated files for all LlamaSearchAI repos..."
./llamasearch-manager generate
EOF
chmod +x llama-generate.sh

# Quick analyze script
cat > llama-analyze.sh << 'EOF'
#!/bin/bash
echo "🦙 Analyzing LlamaSearchAI ecosystem..."
./llamasearch-manager analyze
EOF
chmod +x llama-analyze.sh

# Daily sync script
cat > llama-daily-sync.sh << 'EOF'
#!/bin/bash
echo "🦙 Running daily sync for LlamaSearchAI..."
echo "Started at: $(date)"

# Clone/update all repos
./llamasearch-manager clone

# Generate fresh concatenated files
./llamasearch-manager generate

# Generate documentation
./llamasearch-manager docs

echo "Completed at: $(date)"
EOF
chmod +x llama-daily-sync.sh

# Create a comprehensive run script
cat > llama-full-run.sh << 'EOF'
#!/bin/bash
echo "🦙 LlamaSearchAI Complete Repository Management"
echo "=============================================="

# 1. Scan repositories
echo -e "\n1️⃣ Scanning repositories..."
./llamasearch-manager scan

# 2. Clone/update all
echo -e "\n2️⃣ Cloning/updating repositories..."
./llamasearch-manager clone

# 3. Generate concatenated files
echo -e "\n3️⃣ Generating concatenated files..."
./llamasearch-manager generate

# 4. Analyze ecosystem
echo -e "\n4️⃣ Analyzing ecosystem..."
./llamasearch-manager analyze

# 5. Generate documentation
echo -e "\n5️⃣ Generating documentation..."
./llamasearch-manager docs

echo -e "\n✅ Complete! Check the output directory for generated files."
EOF
chmod +x llama-full-run.sh

# Test the setup
echo -e "\n${GREEN}Testing setup...${NC}"
python3 -c "from src.llamasearch_manager import LlamaSearchManager; print('✅ Python imports working')"

# Display summary
echo -e "\n${GREEN}======================================"
echo "✅ LlamaSearchAI Manager Setup Complete!"
echo "======================================${NC}"

echo -e "\n📋 Available Commands:"
echo "  ./llamasearch-manager scan      - Scan all repos"
echo "  ./llamasearch-manager clone     - Clone/update repos"
echo "  ./llamasearch-manager generate  - Generate concatenated files"
echo "  ./llamasearch-manager analyze   - Analyze ecosystem"
echo "  ./llamasearch-manager create    - Create new project"
echo "  ./llamasearch-manager docs      - Generate documentation"
echo "  ./llamasearch-manager dashboard - Launch web dashboard"

echo -e "\n🚀 Quick Scripts:"
echo "  ./llama-scan.sh       - Quick repository scan"
echo "  ./llama-generate.sh   - Generate all concatenated files"
echo "  ./llama-analyze.sh    - Quick ecosystem analysis"
echo "  ./llama-daily-sync.sh - Run daily sync routine"
echo "  ./llama-full-run.sh   - Run complete workflow"

echo -e "\n💡 Getting Started:"
echo "1. Set your GitHub token (if not already set):"
echo "   export GITHUB_TOKEN='your_token'"
echo ""
echo "2. Run a quick scan:"
echo "   ./llama-scan.sh"
echo ""
echo "3. Generate concatenated files for all repos:"
echo "   ./llama-generate.sh"
echo ""
echo "4. Or run everything at once:"
echo "   ./llama-full-run.sh"

echo -e "\n📁 Output Locations:"
echo "  Repositories: ~/Development/llamasearchai/"
echo "  Exports: ~/Documents/llamasearchai_exports/"
echo "  Logs: ~/.llamasearch/logs/"

if [ -z "$GITHUB_TOKEN" ]; then
    echo -e "\n${YELLOW}⚠️  Remember to set your GITHUB_TOKEN for full functionality!${NC}"
fi