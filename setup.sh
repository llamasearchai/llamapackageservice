#!/usr/bin/env bash
# setup.sh - Automated installation and setup script for Package Processor
# This script is designed to run on an M3 Max MacBook Pro (macOS / ARM).

set -e

# ANSI color codes
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Package Processor Setup & Installation - Ultimate Edition${NC}"
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"

# Build the project
echo -e "${GREEN}Building project...${NC}"
cargo build --release

# Create necessary directories if they don't exist
echo -e "${GREEN}Creating directory structure...${NC}"
mkdir -p {input,output}/{github_repos,github_orgs,pypi_packages,pypi_profiles,crates}

# Set up default configuration if not present
if [ ! -f config.json ]; then
    echo -e "${GREEN}Creating default configuration...${NC}"
    cat > config.json << EOF
{
    "output_dir": "output",
    "input_dir": "input",
    "github_token": null,
    "pypi_timeout": 30,
    "max_concurrent_downloads": 3
}
EOF
fi

echo -e "${GREEN}Setup completed successfully!${NC}" 