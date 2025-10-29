#!/bin/bash
set -e

echo "🔧 Fixing code errors..."

# Determine the correct sed in-place syntax based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS requires an argument (empty string) after -i
    SED_INPLACE="sed -i ''"
else
    # Linux and others just use -i
    SED_INPLACE="sed -i"
fi

# Fix fs name conflicts
find src -name "*.rs" -type f -exec $SED_INPLACE 's/use tokio::fs;/use tokio::fs as tokio_fs;/g' {} \;

# Fix Result type usage
find src -name "*.rs" -type f -exec $SED_INPLACE 's/Result<\([^,]*\), ProcessorError>/Result<\1>/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/Result::<_, std::io::Error>::Ok/Result::Ok/g' {} \;

# Fix tokio fs vs std fs confusion - remove .await from std fs calls
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::read_to_string(\(.*\))\.await/fs::read_to_string(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::write(\(.*\))\.await/fs::write(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::remove_file(\(.*\))\.await/fs::remove_file(\1)/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::create_dir_all(\(.*\))\.await/fs::create_dir_all(\1)/g' {} \;

# Fix tokio fs calls that need .await
find src -name "*.rs" -type f -exec $SED_INPLACE 's/tokio_fs::read_to_string(\(.*\))/tokio_fs::read_to_string(\1).await/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/tokio_fs::write(\(.*\))/tokio_fs::write(\1).await/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/tokio_fs::remove_file(\(.*\))/tokio_fs::remove_file(\1).await/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/tokio_fs::create_dir_all(\(.*\))/tokio_fs::create_dir_all(\1).await/g' {} \;

# Create a proper Cache struct and impl block
cat > src/cache_fixed.rs << 'EOF'
use crate::error::Result;
use crate::cache::CacheMetadata;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::fs;

pub struct Cache {
    base_dir: PathBuf,
}

impl Cache {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let path = self.get_path(key);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        Ok(Some(content))
    }

    pub async fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.get_path(key);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let path = self.get_path(key);
        
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        fs::write(&path, value)?;
        Ok(())
    }

    pub async fn set_with_metadata(&self, key: &str, value: &str, metadata: &CacheMetadata) -> Result<()> {
        self.set(key, value).await?;
        
        let meta_path = self.get_metadata_path(key);
        let entry = CacheEntry {
            url: key.to_string(),
            path: self.get_path(key),
            timestamp: Utc::now(),
            ttl: metadata.ttl,
        };
        
        fs::write(&meta_path, serde_json::to_string(&entry)?)?;
        Ok(())
    }

    fn get_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.cache", key))
    }

    fn get_metadata_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.meta", key))
    }
}

pub struct CacheEntry {
    pub url: String,
    pub path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub ttl: u64,
}
EOF

# Replace the cache.rs file with our fixed version
if [ -f "src/cache.rs" ]; then
    mv src/cache_fixed.rs src/cache.rs
fi

# Add missing imports
find src -name "registry.rs" -type f -exec $SED_INPLACE '1s/^/use crate::Config;\n/' {} \;
find src -name "common.rs" -type f -exec $SED_INPLACE '1s/^/use std::io::Read;\n/' {} \;

# Fix ReadDir streaming issues - replace with a proper async solution
find src -name "*.rs" -type f -exec $SED_INPLACE 's/\.filter_map(|r| r\.ok())/\.into_iter().filter_map(|r| r.ok())/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::read_dir(\([^)]*\))\.await?/tokio_fs::read_dir(\1).await?.collect::<Vec<_>>().await/g' {} \;

# Fix indexing into RepoInfo struct
find src -name "github.rs" -type f -exec $SED_INPLACE 's/repo_info\["description"\]/repo_info.description/g' {} \;
find src -name "github.rs" -type f -exec $SED_INPLACE 's/repo_info\["stargazers_count"\]/repo_info.stargazers_count/g' {} \;
find src -name "github.rs" -type f -exec $SED_INPLACE 's/repo_info\["forks_count"\]/repo_info.forks_count/g' {} \;
find src -name "github.rs" -type f -exec $SED_INPLACE 's/repo_details\["topics"\]/repo_details.topics/g' {} \;
find src -name "github.rs" -type f -exec $SED_INPLACE 's/repo_info\.default_branch/repo_info["default_branch"]/g' {} \;

# Fix missing .await on futures
find src -name "*.rs" -type f -exec $SED_INPLACE 's/\.map_err(|e| ProcessorError::IO(e))?/\.map_err(|e| ProcessorError::IO(e)).await?/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/fs::write(output_path, bytes)?/fs::write(output_path, bytes).await?/g' {} \;

# Fix path borrowing issue
find src -name "github.rs" -type f -exec $SED_INPLACE 's/self.build_file_tree(path,/self.build_file_tree(\&path,/g' {} \;

# Fix registry lifetime issue
find src -name "registry.rs" -type f -exec $SED_INPLACE 's/-> Option<Box<dyn PackageProcessor>>/-> Option<Box<dyn PackageProcessor + '\''_>>/g' {} \;

# Fix unused variables warnings
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let mut outfile =/let mut _outfile =/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let result =/let _result =/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let json =/let _json =/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/pb: ProgressBar/pb: _ProgressBar/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/pb: \&ProgressBar/pb: \&_ProgressBar/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/client: \&Client/client: \&_Client/g' {} \;
find src -name "*.rs" -type f -exec $SED_INPLACE 's/let mut config =/let config =/g' {} \;

# Remove duplicate module file
if [ -f "src/rate_limiter.rs" ] && [ -f "src/rate_limiter/mod.rs" ]; then
  rm "src/rate_limiter/mod.rs"
  echo "Removed duplicate rate_limiter module file"
fi

# Run cargo check to see if we fixed the issues
cargo check

echo "✅ Fix script completed!" 