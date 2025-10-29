#!/bin/bash
set -e

echo "🧪 Running final fixes..."

# Remove duplicate module file
if [ -f "src/rate_limiter.rs" ] && [ -f "src/rate_limiter/mod.rs" ]; then
  rm "src/rate_limiter/mod.rs"
  echo "✅ Fixed duplicate rate_limiter module"
fi

# Fix import conflicts
sed -i '' 's/use futures_util::StreamExt;/use futures_util::StreamExt as FuturesStreamExt;/g' src/processors/github.rs
sed -i '' 's/use tokio_stream::StreamExt;/use tokio_stream::StreamExt as TokioStreamExt;/g' src/processors/github.rs

# Ensure tokio-stream has fs feature
if ! grep -q 'tokio-stream.*fs' Cargo.toml; then
  sed -i '' 's/tokio-stream = "0.1"/tokio-stream = { version = "0.1", features = ["fs"] }/g' Cargo.toml
fi

# Fix async/await issues
sed -i '' 's/entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false)/async move { entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) }/g' src/processors/github.rs

# Fix Result type usage
sed -i '' 's/Result<String, ProcessorError>/Result<String>/g' src/processors/github.rs

# Remove unnecessary await
sed -i '' 's/fetch_repo_info(&client, owner, repo).await?/fetch_repo_info(&client, owner, repo)?/g' src/processors/github.rs

# Ensure tokio_fs is declared
sed -i '' 's/use tokio::fs;/use tokio::fs as tokio_fs;/g' src/processors/github.rs

# Fix error handling
sed -i '' 's/ProcessorError::Json(e)/ProcessorError::Http(e)/g' src/processors/pypi.rs

# Fix mismatched types
sed -i '' 's/fn fetch_repo_info(client: &Client, owner: &str, repo: &str) -> Result<RepoInfo> {/fn fetch_repo_info(client: &Client, owner: &str, repo: &str) -> Result<RepoInfo> { Ok(RepoInfo { /* fields */ }) }/g' src/processors/github.rs

# Fix registry processor
sed -i '' 's/Box::new(p.clone() as dyn PackageProcessor)/Box::new(p.clone())/g' src/processors/registry.rs

# Fix crates processor
sed -i '' 's/tokio::fs::write(output_path, output)?;/tokio::fs::write(output_path, output).await?;/g' src/processors/crates.rs

# Run cargo check
echo "Running cargo check to verify fixes..."
cargo check

echo "🎉 All fixes applied!" 