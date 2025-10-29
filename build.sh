#!/bin/bash
set -e

echo "🧪 Running build and test cycle..."

# Clean previous build artifacts
cargo clean

# Update dependencies
cargo update

# Build in release mode with all features
cargo build --release --all-features

# Run the tests
cargo test --all-features -- --nocapture

# Run the program
cargo run --release

echo "✅ Build and test cycle completed!" 