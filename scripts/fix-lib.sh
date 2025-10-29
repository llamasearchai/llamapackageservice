#!/bin/bash
set -e

echo "🔧 Fixing library exports..."

cat > src/lib.rs << 'EOL'
pub mod processors;
pub mod error;
pub mod cache;
pub mod extended_processors;
pub mod config;

pub use config::Config;
pub use error::{ProcessorError, Result};
pub use cache::{Cache, CacheMetadata};
pub use processors::registry::ProcessorRegistry;
pub use processors::PackageProcessor;
EOL

echo "✅ Library exports fixed!"
