#!/usr/bin/env bash
set -e

echo "Checking documentation..."

# Run documentation tests
cargo test --doc

# Build documentation
cargo doc --no-deps --all-features

# Check for common documentation issues
find src -name "*.rs" -exec grep -l "^//! " {} \; | while read -r file; do
    # Ensure only one module documentation block exists
    if [ $(grep -c "^//! #" "$file") -gt 1 ]; then
        echo "Error: Multiple module documentation blocks found in $file"
        exit 1
    fi
done

echo "Documentation check completed successfully!" 