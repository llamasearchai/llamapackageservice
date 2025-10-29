#!/usr/bin/env bash
set -e

echo "🧪 Running test suite..."

# Run cargo tests with all features
cargo test --all-features -- --nocapture

# Run specific test scenarios
echo "Testing GitHub repository processing..."
cargo test --test github_tests -- --nocapture

echo "Testing PyPI package processing..."
cargo test --test pypi_tests -- --nocapture

# Run integration tests
echo "Running integration tests..."
cargo test --test '*' -- --nocapture

echo "✅ All tests completed successfully!" 