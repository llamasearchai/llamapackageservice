#!/bin/bash
set -e

echo "🔧 Fixing package processor interface..."

cat > src/processors/mod.rs << 'EOL'
pub mod github;
pub mod common;
pub mod registry;

use std::path::Path;
use async_trait::async_trait;
use crate::Config;
use crate::error::Result;

#[async_trait]
pub trait PackageProcessor: Send + Sync {
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()>;
    fn name(&self) -> &'static str;
    fn accepts(&self, url: &str) -> bool;
}
EOL

echo "✅ Package processor interface fixed!" 