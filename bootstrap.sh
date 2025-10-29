#!/usr/bin/env bash
# bootstrap.sh - Ultimate one-step script to set up and run LlamaSearch

set -e

# ANSI color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

# Llama ASCII art
print_llama() {
    echo -e "${CYAN}"
    echo "         __      _                     "
    echo "       /\\ \\__  /\\ \\__                "
    echo "  _____\\ \\ ,_\\/  \\ ,_\\   LlamaSearch    "
    echo "_______\\ \\ \\/____\\ \\ \\/___ Package Analyzer"
    echo "         \\ \\__\\   \\ \\__\\                "
    echo -e "${NC}"
}

print_llama

echo -e "${GREEN}=== LlamaSearch Bootstrap ===${NC}"

# Check for Rust installation
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Update Rust if installed
rustup update

# Clean any previous builds
echo -e "${GREEN}Cleaning previous builds...${NC}"
cargo clean

# Create project structure
echo -e "${GREEN}Creating directory structure...${NC}"
mkdir -p {input,output}/{github_repos,github_orgs,pypi_packages,pypi_profiles,rust_crates}

# Set up default configuration if it doesn't exist
if [ ! -f config.json ]; then
    echo -e "${GREEN}Creating default configuration...${NC}"
    cat > config.json << EOF
{
    "output_dir": "output",
    "input_dir": "input",
    "github_token": null,
    "pypi_timeout": 30,
    "max_concurrent_downloads": 3,
    "ollama_model": "deepseek-r1:70b"
}
EOF
fi

# Check for Ollama installation and the required model
if ! command -v ollama &> /dev/null; then
    echo -e "${RED}Error: Ollama is not installed. Please install it from https://ollama.ai${NC}"
    exit 1
fi

echo -e "${GREEN}Checking for deepseek-r1 model...${NC}"
if ! ollama list | grep -q "deepseek-r1"; then
    echo -e "${YELLOW}Downloading deepseek-r1 model...${NC}"
    ollama pull deepseek-r1:70b
fi

# Run the setup script
echo -e "${GREEN}Running setup script...${NC}"
chmod +x setup.sh
./setup.sh

echo -e "${GREEN}Bootstrap completed successfully!${NC}"
echo -e "${CYAN}Welcome to LlamaSearch - Your Intelligent Package Analyzer${NC}"
