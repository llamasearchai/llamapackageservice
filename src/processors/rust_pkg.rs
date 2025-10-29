use crate::error::ProcessorError;
use crate::processors::common::{self, setup_progress_style};
use indicatif::ProgressBar;
use reqwest::Client;
use serde_json::Value;
use std::path::Path;

/// Process a Rust crate from a crates.io URL.
/// It downloads the crates.io page, extracts the GitHub repository URL,
/// downloads the repository archive (assumed from the "main" branch),
/// extracts the source code, generates an index file, and writes it to the rust_packages folder.
pub async fn process_rust_crate(
    crate_url: &str,
    output_dir: &Path,
    pb: &ProgressBar
) -> Result<(), ProcessorError> {
    setup_progress_style(pb);
    pb.set_message("Processing Rust crate...");

    // Extract crate name and get GitHub URL
    let crate_name = crate_url.split('/')
        .last()
        .unwrap_or("unknown")
        .to_string();
        
    let github_url = get_crate_github_url(&crate_name).await?;

    // Download and process the GitHub repository
    let archive_bytes = common::download_github_repo(&github_url, pb).await?;
    
    let temp_dir = tempfile::tempdir()
        .map_err(|e| ProcessorError::IO(e))?;
    
    common::extract_zip_archive(&archive_bytes, temp_dir.path(), pb)?;

    // Generate index
    pb.set_message("Generating crate index...");
    let mut master_index = String::new();
    master_index.push_str(&format!("# Rust Crate: {}\n\n", crate_name));
    master_index.push_str(&format!("Source: {}\n", crate_url));
    master_index.push_str(&format!("GitHub: {}\n\n", github_url));

    let repo_index = crate::processors::github::generate_repo_index(temp_dir.path())?;
    master_index.push_str(&repo_index);

    // Save the index
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let file_name = format!("{}_rust_{}.txt", crate_name, timestamp);
    let file_path = output_dir.join(file_name);
    
    std::fs::write(file_path, master_index)
        .map_err(|e| ProcessorError::IO(e))?;

    pb.finish_with_message(format!("✨ Rust crate {} processed successfully", crate_name));
    Ok(())
}

async fn get_crate_github_url(crate_name: &str) -> Result<String, ProcessorError> {
    let client = Client::new();
    let api_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    
    let response = client.get(&api_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;

    let data: Value = response.json()
        .await
        .map_err(|e| ProcessorError::Processing(e.to_string()))?;

    if let Some(repo_url) = data["crate"]["repository"].as_str() {
        Ok(repo_url.to_string())
    } else {
        Err(ProcessorError::Processing("GitHub repository not found".to_string()))
    }
} 