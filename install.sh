#!/usr/bin/env bash
# install.sh - Installation script for the Ultimate Repo Processor

set -e

print_message() {
    echo -e "\033[1;32m$1\033[0m"
}

print_message "Installing Rust and Cargo..."
if ! command -v rustc &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    print_message "Rust is already installed."
fi

print_message "Installing Git, jq, and unzip..."
if ! command -v git &>/dev/null; then
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update && sudo apt-get install -y git jq unzip
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        if ! command -v brew &>/dev/null; then
            print_message "Homebrew is not installed. Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        fi
        brew install git jq unzip
    else
        echo "Unsupported OS. Please install git, jq, and unzip manually."
        exit 1
    fi
else
    print_message "Git is already installed."
fi

# Create output directory if it doesn't exist
OUTPUT_DIR="output"
if [ ! -d "$OUTPUT_DIR" ]; then
    mkdir "$OUTPUT_DIR"
    print_message "Output directory created at $OUTPUT_DIR."
else
    print_message "Output directory already exists."
fi

print_message "Building the Rust program (release mode)..."
cargo build --release

print_message "Running the Rust program..."
./target/release/repo_processor
