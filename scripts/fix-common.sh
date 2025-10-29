#!/bin/bash
set -e

echo "🔧 Fixing common utilities..."

cat > src/processors/common.rs << 'EOL'
use std::path::{Path, PathBuf};
use std::io::Read;
use tokio::fs as tokio_fs;
use std::fs;
use tokio::io::AsyncWriteExt;
use reqwest::Client;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crate::error::{ProcessorError, Result};

pub async fn download_file(url: &str, output_path: &Path) -> Result<()> {
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Download(
            format!("Failed to download {}: HTTP {}", url, response.status())
        ));
    }
    
    let bytes = response.bytes()
        .await
        .map_err(|e| ProcessorError::Network(e))?;
        
    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            tokio_fs::create_dir_all(parent).await?;
        }
    }
    
    tokio_fs::write(output_path, bytes).await?;
    
    Ok(())
}

pub async fn extract_zip_to_directory(zip_path: &Path, output_dir: &Path) -> Result<()> {
    // Create the output directory
    tokio_fs::create_dir_all(output_dir).await?;
    
    // Use std::fs for zip extraction as zip-rs doesn't have async support
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.name());
        
        if file.name().ends_with('/') {
            // Directory
            fs::create_dir_all(&outpath)?;
        } else {
            // File
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            
            let mut _outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut _outfile)?;
        }
    }
    
    Ok(())
}

pub async fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Api(
            format!("API returned error status: {}", response.status())
        ));
    }
    
    response.json::<T>()
        .await
        .map_err(|e| ProcessorError::Parsing(format!("Failed to parse JSON: {}", e)))
}

pub async fn fetch_text(url: &str) -> Result<String> {
    let client = Client::new();
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Api(
            format!("API returned error status: {}", response.status())
        ));
    }
    
    response.text()
        .await
        .map_err(|e| ProcessorError::Parsing(format!("Failed to get text: {}", e)))
}

// Example functions showing correct patterns
pub async fn example_read_file() -> std::io::Result<()> {
    let _result = tokio_fs::read_to_string("some_file.txt").await?;
    Ok(())
}

pub async fn example_json_request() -> Result<()> {
    let client = Client::new();
    let response = client.get("https://api.example.com/data")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e))?;
        
    let _json = response.json::<serde_json::Value>().await
        .map_err(|e| ProcessorError::Parsing(format!("Failed to parse JSON: {}", e)))?;
        
    Ok(())
}

pub async fn read_zip_file(zip_file: &mut zip::read::ZipFile<'_>) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    zip_file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
EOL

echo "✅ Common utilities fixed!"
