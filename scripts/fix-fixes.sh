#!/bin/bash
set -e

echo "🔧 Running all code2txt repository fixes..."

# Apply all fixes above
bash fix_cache.sh
bash fix_github.sh
bash fix_registry.sh
bash fix_package_processor.sh
bash fix_common.sh
bash fix_error.sh
bash fix_config.sh
bash fix_lib.sh

# Run cargo check to verify fixes
echo "🧪 Running cargo check to verify fixes..."
cargo check

if [ $? -eq 0 ]; then
    echo "✅ All fixes were applied successfully!"
else
    echo "❌ Some issues remain. Please check the output above."
fi

