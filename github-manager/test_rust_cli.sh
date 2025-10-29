#!/bin/bash

# Test script for Rust CLI

set -e

echo "🔧 Testing Rust CLI for LlamaSearchAI GitHub Manager..."
echo ""

cd rust-cli

echo "📋 Running cargo check..."
cargo check --all-features

echo ""
echo "🧪 Running unit tests..."
cargo test --lib -- --nocapture

echo ""
echo "🧪 Running integration tests..."
cargo test --test '*' -- --nocapture

echo ""
echo "📊 Running with code coverage (if tarpaulin is installed)..."
if command -v cargo-tarpaulin &> /dev/null; then
    cargo tarpaulin --out Html --output-dir ../coverage
    echo "Coverage report generated in ../coverage/tarpaulin-report.html"
else
    echo "Skipping coverage (install with: cargo install cargo-tarpaulin)"
fi

echo ""
echo "🔍 Running clippy lints..."
cargo clippy -- -D warnings

echo ""
echo "📝 Checking formatting..."
cargo fmt -- --check

echo ""
echo "📦 Building release binary..."
cargo build --release

echo ""
echo "✅ All tests passed!"
echo ""
echo "Binary location: target/release/llamasearch"