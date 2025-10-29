use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use colored::*;
use tracing::info;
use crate::error::{ProcessorError, Result};
use crate::processors::common;
use crate::processors::common::setup_progress_style;
use crate::processors::github::process_github_content;
use async_trait::async_trait;
use indicatif::ProgressBar;
use std::path::Path;
use chrono::Utc;
use crate::processors::PackageProcessor;
use reqwest::Client;
use tempfile::tempdir;
use std::fs;
use serde_json::Value;
use flate2::read::GzDecoder;
use tar::Archive;
use std::io::Read;
use crate::config::Config;
use zip;
use tokio;
use futures_util::TryFutureExt;
use url::Url;
use tokio::io::AsyncWriteExt;

/// Information about a Rust crate
///
/// This struct holds metadata about a Rust crate, including its name,
/// version, description, and related links.
#[derive(Debug, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Name of the Rust crate
    pub name: String,
    /// Version of the crate
    pub version: String,
    /// Optional description of the crate
    pub description: Option<String>,
    /// Optional URL to the crate's documentation
    pub documentation: Option<String>,
    /// Optional URL to the crate's source repository
    pub repository: Option<String>,
    /// URL to download the crate package
    pub download_url: String,
}

impl CrateInfo {
    /// Creates a CrateInfo instance from a URL
    ///
    /// Extracts information from the provided URL to initialize a CrateInfo struct.
    ///
    /// # Arguments
    /// * `url` - The URL of the crate, typically from crates.io
    ///
    /// # Returns
    /// A Result containing the CrateInfo or an error
    pub async fn from_url(url: &str) -> Result<Self> {
        let parts: Vec<&str> = url.split('/').collect();
        let name = parts.last()
            .ok_or_else(|| ProcessorError::Validation("Invalid crate URL".to_string()))?;

        Ok(CrateInfo {
            name: name.to_string(),
            version: "".to_string(),
            description: None,
            documentation: None,
            repository: None,
            download_url: url.to_string(),
        })
    }

    /// Fetches documentation for this crate
    ///
    /// # Returns
    /// A Result containing the documentation content as a String or an error
    pub async fn fetch_docs(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let response = client.get(&self.download_url).send().await?;

        if !response.status().is_success() {
            return Err(ProcessorError::Validation("No documentation available".to_string()));
        }

        Ok(response.text().await?)
    }
}

/// Downloads a Rust crate from its URL
///
/// # Arguments
/// * `crate_info` - Information about the crate to download
/// * `output_path` - Path where the downloaded crate should be saved
///
/// # Returns
/// Result indicating success or failure of the download operation
pub async fn download_crate(crate_info: &CrateInfo, output_path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(&crate_info.download_url).send().await?;

    if !response.status().is_success() {
        return Err(ProcessorError::Download(format!(
            "Failed to download crate: HTTP {}",
            response.status()
        )));
    }

    let bytes = response.bytes().await?;
    tokio::fs::write(output_path, &bytes).await?;

    Ok(())
}

/// Extracts a downloaded crate archive
///
/// # Arguments
/// * `crate_path` - Path to the downloaded crate archive
/// * `extract_path` - Directory where the crate should be extracted
///
/// # Returns
/// Result indicating success or failure of the extraction operation
pub async fn extract_crate(crate_path: &str, extract_path: &str) -> Result<()> {
    let file = std::fs::File::open(crate_path)?;
    let reader = std::io::BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = std::path::Path::new(extract_path).join(file.name());

        if file.name().ends_with('/') {
            tokio::fs::create_dir_all(&outpath).await?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    tokio::fs::create_dir_all(p).await?;
                }
            }
            let mut outfile = tokio::fs::File::create(&outpath).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            outfile.write_all(&buffer).await?;
        }
    }

    Ok(())
}

/// Processor for handling Rust crates
///
/// This processor downloads, extracts, and analyzes Rust crates
/// from crates.io or other sources.
pub struct RustCrateProcessor {
    /// HTTP client for making requests
    client: Client,
}

impl RustCrateProcessor {
    /// Creates a new RustCrateProcessor with default configuration
    ///
    /// # Returns
    /// A new RustCrateProcessor instance
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Clone for RustCrateProcessor {
    fn clone(&self) -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl PackageProcessor for RustCrateProcessor {
    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()> {
        config.validate().await?;
        process_rust_crate(url, output_dir, &ProgressBar::new_spinner()).await
    }

    fn name(&self) -> &'static str {
        "Rust Crate"
    }

    fn accepts(&self, url: &str) -> bool {
        url.contains("crates.io")
    }

    async fn validate(&self, url: &str) -> Result<()> {
        if !url.contains("crates.io") {
            return Err(ProcessorError::Validation(format!("Invalid crates.io URL: {}", url)));
        }
        Ok(())
    }
}

/// Processes a Rust crate from a crates.io URL by:
/// - Fetching crate metadata from the Crates.io API
/// - Downloading the crate archive (.crate file, which is a gzipped tar)
/// - Extracting the archive contents
/// - Indexing selected files and dependencies
/// - Saving all this information into an output file.
pub async fn process_rust_crate(
    url: &str,
    output_dir: &Path,
    pb: &ProgressBar
) -> Result<()> {
    setup_progress_style(pb);
    pb.set_message("Processing Rust crate...");

    // Extract crate name from the URL (for example, "serde" from "https://crates.io/crates/serde")
    let crate_name = url.split('/')
        .last()
        .ok_or_else(|| ProcessorError::Validation("Invalid crates.io URL".into()))?;

    // Fetch crate metadata from the Crates.io API
    let client = Client::new();
    let api_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let response = client.get(&api_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
    let data: Value = response.json()
        .await
        .map_err(|e| ProcessorError::Processing(e.to_string()))?;

    // Start composing the output content with crate metadata:
    let mut content = String::new();
    content.push_str(&format!("# Rust Crate: {}\n\n", crate_name));
    content.push_str(&format!("Source: {}\n", url));

    // Append metadata if available
    if let Some(crate_data) = data.get("crate") {
        if let Some(version) = crate_data.get("max_version").and_then(|v| v.as_str()) {
            content.push_str(&format!("Version: {}\n", version));
        }
        if let Some(desc) = crate_data.get("description").and_then(|v| v.as_str()) {
            content.push_str(&format!("\nDescription: {}\n", desc));
        } else {
            content.push_str("\nDescription: No description available\n");
        }
        if let Some(downloads) = crate_data.get("downloads").and_then(|v| v.as_u64()) {
            content.push_str(&format!("Downloads: {}\n", downloads));
        }
        if let Some(doc) = crate_data.get("documentation").and_then(|v| v.as_str()) {
            content.push_str(&format!("Documentation: {}\n", doc));
        }
    }

    // If a version is available, download the crate archive.
    if let Some(version) = data["crate"]["max_version"].as_str() {
        let download_url = format!(
            "https://crates.io/api/v1/crates/{}/{}/download", crate_name, version
        );

        pb.set_message("Downloading crate archive...");

        let response = client.get(&download_url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
        let bytes = response.bytes()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        // Create a temporary directory for extracting the archive.
        let temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;

        // Create a GzDecoder to decompress the archive (a gzipped tar file)
        let gz_decoder = GzDecoder::new(&bytes[..]);
        let mut archive = Archive::new(gz_decoder);

        // Unpack the archive into the temporary directory.
        archive.unpack(temp_dir.path())
            .map_err(|e| ProcessorError::IO(e))?;

        // Append a package contents section.
        content.push_str("\n## Package Contents:\n\n");

        // Iterate over the files extracted and include content for important files.
        for entry_result in fs::read_dir(temp_dir.path()).map_err(|e| ProcessorError::IO(e))? {
            let entry = entry_result.map_err(|e| ProcessorError::IO(e))?;
            let path = entry.path();

            if path.is_file() {
                // Determine the path relative to the temporary extraction directory.
                let relative_path = path.strip_prefix(temp_dir.path())
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                content.push_str(&format!("### {}\n", relative_path));

                // Include content if the file is deemed important
                if should_include_file(&relative_path) {
                    if let Ok(file_content) = fs::read_to_string(&path) {
                        content.push_str("```rust\n");
                        content.push_str(&file_content);
                        content.push_str("\n```\n\n");
                    }
                }
            }
        }

        // Append the dependencies section (if available from the first version entry)
        if let Some(deps) = data.get("versions")
                                 .and_then(|versions| versions.get(0))
                                 .and_then(|version_obj| version_obj.get("dependencies"))
                                 .and_then(|deps| deps.as_array()) {
            content.push_str("\n## Dependencies:\n\n");
            for dep in deps {
                if let (Some(name), Some(req)) = (
                    dep.get("crate_id").and_then(|v| v.as_str()),
                    dep.get("req").and_then(|v| v.as_str())
                ) {
                    content.push_str(&format!("- {} = \"{}\"\n", name, req));
                }
            }
        }

        // Append GitHub repository content if available
        if let Some(repo_url) = data["crate"]["repository"].as_str() {
            if repo_url.contains("github.com") {
                match process_github_content(repo_url, temp_dir.path()).await {
                    Ok(github_content) => {
                        content.push_str("\n## GitHub Repository Content\n\n");
                        content.push_str(&github_content);
                    }
                    Err(e) => {
                        info!("Failed to process GitHub repository: {}", e);
                    }
                }
            }
        }
    }

    // Save the complete index file using the common helper.
    let output_path = output_dir.join(format!("{}.md", crate_name));
    common::save_output_file(&content, &output_path).await?;

    pb.finish_with_message(format!("✨ Rust crate {} processed successfully", crate_name));
    Ok(())
}

/// Returns true if the file at the given path should be included in the output index.
fn should_include_file(path: &str) -> bool {
    let important_files = [
        "Cargo.toml",
        "Cargo.lock",
        "README.md",
        "LICENSE",
        "CHANGELOG.md",
        "src/lib.rs",
        "src/main.rs",
    ];
    important_files.iter().any(|f| path.ends_with(f))
        || path.starts_with("src/")
        || path.ends_with(".rs")
}

/// Processor for Rust crates from crates.io
///
/// This processor handles the downloading, extraction, and analysis of
/// Rust crates from the crates.io registry.
pub struct CratesProcessor {
    client: Client,
}

impl CratesProcessor {
    /// Creates a new CratesProcessor with default configuration
    ///
    /// # Returns
    /// A new CratesProcessor instance
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }

    async fn validate_url(&self, url: &str) -> Result<()> {
        if !url.contains("crates.io/crates/") {
            return Err(ProcessorError::new("Not a valid Crates.io URL"));
        }
        Ok(())
    }

    async fn get_crate_info(&self, crate_name: &str) -> Result<serde_json::Value> {
        let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        let response = self.client.get(&url)
            .send()
            .await?;
            
        let data = response.json::<serde_json::Value>()
            .await?;
            
        Ok(data)
    }
}

#[async_trait]
impl PackageProcessor for CratesProcessor {
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        setup_progress_style(&pb);
        
        // Extract crate name
        let crate_name = extract_crate_name_from_url(url)?;
        
        // Process directly
        process_crate_by_name(&crate_name, output_dir, &pb).await?;
        
        pb.finish_with_message(format!("✨ Rust crate {} processed successfully", crate_name));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Crates.io"
    }

    fn accepts(&self, url: &str) -> bool {
        url.contains("crates.io")
    }

    async fn validate(&self, url: &str) -> Result<()> {
        self.validate_url(url).await
    }
}

// Add helper function for crate name extraction
fn extract_crate_name_from_url(url: &str) -> Result<String> {
    let parts: Vec<&str> = url.split('/').collect();
    let name = parts.last()
        .ok_or_else(|| ProcessorError::Validation("Invalid crate URL".to_string()))?;
    Ok(name.to_string())
}

/// Processes a Rust crate from a URL
///
/// This function handles the complete processing of a Rust crate, including
/// downloading the package, extracting it, and generating documentation.
///
/// # Arguments
/// * `url` - The URL of the crate to process
/// * `output_dir` - Directory where output files should be saved
/// * `pb` - Progress bar for displaying status
///
/// # Returns
/// Result indicating success or failure of the processing operation
pub async fn process_crate(url: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Processing Rust crate: {}", url));
    
    let crate_name = extract_crate_name_from_url(url)?;
    process_crate_by_name(&crate_name, output_dir, &pb).await?;
    
    Ok(())
}

// Add the actual processing function that doesn't create more processors
async fn process_crate_by_name(crate_name: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    // Use the progress bar
    pb.set_message(format!("Processing crate: {}", crate_name));
    
    // Create the client with proper user agent
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
    
    // Use docs.rs which is more reliable
    let docs_url = format!("https://docs.rs/{}", crate_name);
    
    pb.set_message(format!("Fetching docs.rs page for: {}", crate_name));
    
    // Fetch docs.rs HTML page
    let response = client.get(&docs_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    let docs_available = response.status().is_success();
    
    // Also check if we can access the crate info from lib.rs (another documentation site)
    let librs_url = format!("https://lib.rs/crates/{}", crate_name);
    
    pb.set_message(format!("Fetching lib.rs page for: {}", crate_name));
    
    // Fetch lib.rs HTML page
    let librs_response = client.get(&librs_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    let librs_available = librs_response.status().is_success();
    
    // Build markdown content with available information
    let mut content = String::new();
    content.push_str(&format!("# Rust Crate: {}\n\n", crate_name));
    
    // Add basic information
    content.push_str("## Package Information\n\n");
    content.push_str(&format!("- **Name:** {}\n", crate_name));
    
    // Add documentation links if available
    content.push_str("\n## Links\n\n");
    
    if docs_available {
        content.push_str(&format!("- **Documentation:** {}\n", docs_url));
    }
    
    if librs_available {
        content.push_str(&format!("- **Lib.rs Page:** {}\n", librs_url));
    }
    
    content.push_str(&format!("- **Crate Page:** https://crates.io/crates/{}\n", crate_name));
    
    // Try to fetch repository information from common patterns
    content.push_str(&format!("- **Potential GitHub Repository:** https://github.com/rust-lang/{}\n", crate_name));
    content.push_str(&format!("- **Alternative Repository:** https://github.com/{}/{}\n", crate_name, crate_name));
    
    // Add installation instructions
    content.push_str("\n## Installation\n\n");
    content.push_str("Add this to your `Cargo.toml`:\n\n");
    content.push_str("```toml\n");
    content.push_str(&format!("[dependencies]\n{} = \"*\"\n", crate_name));
    content.push_str("```\n\n");
    
    content.push_str("## Usage Example\n\n");
    content.push_str("```rust\n");
    content.push_str(&format!("// Import the {} crate\n", crate_name));
    content.push_str(&format!("use {};\n\n", crate_name));
    content.push_str("fn main() {\n");
    content.push_str(&format!("    // Use the {} crate here\n", crate_name));
    content.push_str("    println!(\"Hello from {}!\");\n");
    content.push_str("}\n");
    content.push_str("```\n");
    
    // Create Rust crates directory
    let rust_crates_dir = output_dir.join("rust_crates");
    tokio::fs::create_dir_all(&rust_crates_dir).await
        .map_err(|e| ProcessorError::IO(e))?;
    
    // Save to file in the correct directory
    let output_path = rust_crates_dir.join(format!("{}.md", crate_name));
    common::save_output_file(&content, &output_path).await?;

    pb.finish_with_message(format!("[SUCCESS] Crate {} processed successfully", crate_name));
    Ok(())
} 