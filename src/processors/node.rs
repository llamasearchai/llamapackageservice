use crate::error::ProcessorError;
use indicatif::ProgressBar;
use reqwest::Client;
use regex::Regex;
use std::io::Cursor;
use std::path::Path;
use tempfile::tempdir;
use zip::ZipArchive;
use time::OffsetDateTime;
use indicatif::ProgressStyle;
use tokio;

/// Process a Node package from an npmjs.com URL.
/// It downloads the npm page, extracts the GitHub repo link,
/// downloads the repository archive (assumed from the "main" branch),
/// extracts it, generates an index using the common function,
/// and writes the index into the node_packages output folder.
pub async fn process_node_package(url: &str, output_dir: &Path, pb: &ProgressBar) -> Result<(), ProcessorError> {
    let client = Client::new();
    let response = client.get(url)
        .send().await.map_err(|e| ProcessorError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!("Failed to fetch Node package page: HTTP {}", response.status())));
    }
    let html = response.text().await.map_err(|e| ProcessorError::Network(e.to_string()))?;
    
    // Extract GitHub repo URL from the npm page.
    let re = Regex::new(r#"href="(https://github\.com/[^"]+)""#)
        .map_err(|e| ProcessorError::Processing(e.to_string()))?;
    let repo_url = if let Some(cap) = re.captures(&html) {
        cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string()
    } else {
        return Err(ProcessorError::Processing("No GitHub repository link found in Node package page".to_string()));
    };
    
    // Download repository archive (assume "main" branch)
    let archive_url = format!("{}/archive/refs/heads/main.zip", repo_url);
    let archive_bytes = client.get(&archive_url)
        .send().await.map_err(|e| ProcessorError::Network(e.to_string()))?
        .bytes().await.map_err(|e| ProcessorError::Network(e.to_string()))?;
    
    // Create temporary directory and extract archive
    let temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;
    let cursor = Cursor::new(archive_bytes);
    let mut zip = ZipArchive::new(cursor).map_err(|e| ProcessorError::Processing(e.to_string()))?;
    zip.extract(temp_dir.path()).map_err(|e| ProcessorError::Processing(e.to_string()))?;
    
    // Determine package name from repo URL
    let package_name = repo_url.split('/').last().unwrap_or("unknown");
    
    // Generate package index using the common function from the PyPI module.
    let index = crate::processors::pypi::generate_package_index(temp_dir.path(), package_name, pb)?;
    
    let timestamp = OffsetDateTime::now_utc().unix_timestamp();
    let file_name = format!("{}_node_{}.txt", package_name, timestamp);
    let file_path = output_dir.join(file_name);
    tokio::fs::write(file_path, index).map_err(|e| ProcessorError::IO(e)).await?;
    
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
            .unwrap()
    );
    pb.set_message(format!("Processing {} package...", package_name));
    
    pb.println(format!("Node package {} processed successfully", package_name));
    Ok(())
} 