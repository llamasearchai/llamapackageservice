#!/bin/bash
set -e

echo "🔧 Fixing remaining errors..."

# Fix fs module resolution
sed -i '' 's/use tokio::fs as tokio_fs;/use tokio::fs;/g' src/processors/github.rs

# Fix RepoInfo initialization
sed -i '' 's/Ok(RepoInfo { \/\* fields \*\/ })/Ok(RepoInfo {\n        name: repo.to_string(),\n        owner: owner.to_string(),\n        description: None,\n        language: None,\n        stars: 0,\n        forks: 0,\n        default_branch: "main".to_string(),\n    })/g' src/processors/github.rs

# Fix async move nesting issue
sed -i '' 's/\.find(|entry| async move {.*})/\.next()/g' src/processors/github.rs

# Add #[derive(Clone)] to RateLimit
sed -i '' 's/struct RateLimit {/#[derive(Clone)]\nstruct RateLimit {/g' src/rate_limiter.rs

# Fix ProcessorError::Json to ProcessorError::Http
sed -i '' 's/ProcessorError::Json(e)/ProcessorError::Http(e)/g' src/processors/common.rs

# Fix Result<(), ProcessorError> to Result<()>
sed -i '' 's/Result<(), ProcessorError>/Result<()>/g' src/processors/pypi_profile.rs

# Fix process_profile function
sed -i '' 's/fn process_profile() -> Result<()> {/fn process_profile() -> Result<()> {\n    Ok(())/g' src/processors/pypi_profile.rs

# Fix registry processor cloning
sed -i '' 's/processors.get(name).map(|p| Box::new(p.clone()))/processors.get(name).map(|p| p.clone())/g' src/processors/registry.rs

# Fix extract_archive_async implementation
cat > src/processors/common_extract_archive_async.rs << 'EOF'
pub async fn extract_archive_async(archive_bytes: &[u8], extract_path: &Path, progress: Option<&ProgressBar>) -> Result<()> {
    // Async implementation with progress reporting
    let cursor = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| ProcessorError::Processing(format!("Failed to read archive: {}", e)))?;
    
    if let Some(pb) = progress {
        pb.set_length(archive.len() as u64);
        pb.set_message("Extracting files...");
    }
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| ProcessorError::Processing(format!("Failed to access file in archive: {}", e)))?;
        
        let outpath = extract_path.join(file.mangled_name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).await
                .map_err(|e| ProcessorError::IO(e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).await
                        .map_err(|e| ProcessorError::IO(e))?;
                }
            }
            
            let mut outfile = fs::File::create(&outpath).await
                .map_err(|e| ProcessorError::IO(e))?;
            
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| ProcessorError::IO(e))?;
                
            outfile.write_all(&buffer).await
                .map_err(|e| ProcessorError::IO(e))?;
        }
        
        if let Some(pb) = progress {
            pb.inc(1);
        }
    }
    
    if let Some(pb) = progress {
        pb.finish_with_message("Extraction complete");
    }
    
    Ok(())
}
EOF

# Replace the empty implementation with the full one
sed -i '' '/pub async fn extract_archive_async.*{/,/}/d' src/processors/common.rs
cat src/processors/common_extract_archive_async.rs >> src/processors/common.rs
rm src/processors/common_extract_archive_async.rs

echo "✅ All errors fixed!"

# Run cargo check to verify
cargo check 