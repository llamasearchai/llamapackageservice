#!/bin/bash
set -e

echo "🔧 Running comprehensive code2txt repository fixes..."

# Create directory structure if it doesn't exist
mkdir -p src/processors
mkdir -p src/extended_processors
mkdir -p src/utils

# Apply all component fixes in the correct order
echo "📦 Fixing error handling..."
bash scripts/fix-error.sh

echo "📦 Fixing configuration..."
bash scripts/fix-config.sh

echo "📦 Fixing package processor interface..."
bash scripts/fix-package-processor.sh

echo "📦 Fixing cache implementation..."
bash scripts/fix-cache.sh

echo "📦 Fixing common utilities..."
bash scripts/fix-common.sh

echo "📦 Fixing GitHub processor..."
bash scripts/fix-github.sh

echo "📦 Fixing processor registry..."
bash scripts/fix-registry.sh

echo "📦 Fixing library exports..."
bash scripts/fix-lib.sh

# Apply the general fixes from fix_errors.sh
echo "🔧 Applying general code fixes..."

# Determine the correct sed in-place syntax based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS requires an argument (empty string) after -i
    SED_INPLACE="sed -i ''"
else
    # Linux and others
    SED_INPLACE="sed -i"
fi

# Fix registry lifetime issue
find src -name "registry.rs" -type f -exec $SED_INPLACE 's/-> Option<Box<dyn PackageProcessor>>/-> Option<Box<dyn PackageProcessor + Send + Sync + '\''static>>/g' {} \;

# Fix unused variables warnings
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let mut client =/let client =/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/client: \&Client/client: \&_Client/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let mut config =/let config =/g' {} \;

# Fix await issues
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::read_to_string(\([^)]*\))\.await/fs::read_to_string(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::write(\([^)]*\))\.await/fs::write(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::remove_file(\([^)]*\))\.await/fs::remove_file(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::create_dir_all(\([^)]*\))\.await/fs::create_dir_all(\1)/g' {} \;

# Fix struct field access
find src -name "*.rs" -type f -exec $SED_INPLACE 's/repo_info\["description"\]/repo_info.description/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/repo_info\["stargazers_count"\]/repo_info.stargazers_count/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/repo_info\["forks_count"\]/repo_info.forks_count/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/repo_details\["topics"\]/repo_details.topics/g' {} \;

# Fix CacheEntry struct field errors
find src -name "*.rs" -type f -exec $SED_INPLACE 's/key: key.to_string()/url: key.to_string()/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/created_at: chrono::Utc::now()/timestamp: chrono::Utc::now()/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/metadata: metadata.clone()/ttl: metadata.ttl/g' {} \;

# Remove duplicate module file
if [ -f "src/rate_limiter.rs" ] && [ -f "src/rate_limiter/mod.rs" ]; then
  rm "src/rate_limiter/mod.rs"
  echo "Removed duplicate rate_limiter module file"
fi

# Run cargo check to see if we fixed the issues
echo "🧪 Running cargo check to verify fixes..."
cargo check

if [ $? -eq 0 ]; then
    echo "✅ All fixes were applied successfully!"
    
    # Run documentation checks
    echo "📚 Checking documentation..."
    bash scripts/check_docs.sh
    
    echo "🚀 Building the project..."
    cargo build
    
    echo "🧪 Running tests..."
    cargo test
    
    echo "✨ Code2txt repository has been successfully fixed and verified!"
else
    echo "❌ Some issues remain. Please check the output above."
    exit 1
fi 