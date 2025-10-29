use crate::error::{ProcessorError, Result};
use reqwest::{Client, StatusCode};
use std::path::Path;
use std::io::Cursor;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::warn;
use tokio::time::sleep;
use std::time::Duration;
use serde_json::Value;
use zip::ZipArchive;
use std::io::Read;
use std::io::Write;
use chrono::Utc;
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

/// Creates a GitHub API client with retry capability
pub fn create_github_client_with_retry(token: Option<&str>) -> Client {
    let mut builder = Client::builder();
    
    // Add authorization if token provided
    if let Some(token) = token {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = format!("token {}", token);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&auth_value).unwrap(),
        );
        builder = builder.default_headers(headers);
    }
    
    // Always include User-Agent
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("code2txt"),
    );
    
    builder.default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Checks if the GitHub API rate limit has been exceeded
pub async fn check_rate_limit(client: &Client) -> Result<()> {
    let response = client.get("https://api.github.com/rate_limit")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    if response.status() == StatusCode::FORBIDDEN {
        return Err(ProcessorError::RateLimitExceeded("GitHub API rate limit exceeded".into()));
    }
    
    let rate_limit = response.json::<Value>().await
        .map_err(|e| ProcessorError::Http(e))?;
        
    let remaining = rate_limit["resources"]["core"]["remaining"]
        .as_u64()
        .unwrap_or(0);
        
    if remaining < 10 {
        warn!("GitHub API rate limit is low: {} requests remaining", remaining);
    }
    
    Ok(())
}

/// Downloads a file from a URL to the specified path
pub async fn download_file(client: &Client, url: &str, output_path: &Path) -> Result<()> {
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to download file: HTTP {}",
            response.status()
        )));
    }
    
    let bytes = response.bytes()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    fs::write(output_path, bytes)
        .await
        .map_err(|e| ProcessorError::IO(e))
}

/// Extracts an archive to the specified directory
pub fn extract_archive(archive_bytes: &[u8], extract_path: &Path) -> Result<()> {
    let reader = Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ProcessorError::Zip(e))?;
        
    archive.extract(extract_path)
        .map_err(|e| ProcessorError::Zip(e))
}

/// Saves content to a file
pub async fn save_output_file(content: &str, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await
            .map_err(|e| ProcessorError::IO(e))?;
    }
    
    fs::write(output_path, content).await
        .map_err(|e| ProcessorError::IO(e))
}

/// Configures a progress bar with a consistent style
pub fn setup_progress_style(pb: &ProgressBar) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_strings(&[
                "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
            ])
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
}

/// Downloads a GitHub repository as a byte array
pub async fn download_github_repo(github_url: &str, pb: &ProgressBar) -> Result<Vec<u8>> {
    pb.set_message(format!("Downloading repository: {}", github_url));
    
    // Extract owner and repo from URL
    let parts: Vec<&str> = github_url.trim_end_matches('/').split('/').collect();
    if parts.len() < 5 {
        return Err(ProcessorError::Validation("Invalid GitHub URL".into()));
    }
    
    let owner = parts[parts.len() - 2];
    let repo = parts[parts.len() - 1];
    
    // Construct archive URL (try main branch first, then master if that fails)
    let archive_url = format!("https://github.com/{}/{}/archive/refs/heads/main.zip", owner, repo);
    
    // Download the archive
    let client = reqwest::Client::new();
    let response = client.get(&archive_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        // Try with 'master' branch if 'main' fails
        let archive_url = format!("https://github.com/{}/{}/archive/refs/heads/master.zip", owner, repo);
        let response = client.get(&archive_url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Network(format!(
                "Failed to download repository: HTTP status {}", 
                response.status()
            )));
        }
        
        let bytes = response.bytes()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?
            .to_vec();
            
        pb.set_message("Download completed");
        return Ok(bytes);
    }
    
    let bytes = response.bytes()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?
        .to_vec();
        
    pb.set_message("Download completed");
    Ok(bytes)
}

/// Extracts a ZIP archive with progress reporting
pub async fn extract_zip_archive(archive_bytes: &[u8], extract_path: &Path, pb: Option<&ProgressBar>) -> Result<()> {
    if let Some(progress) = pb {
        progress.set_message("Extracting archive...");
    }
    
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| ProcessorError::Zip(e))?;
    
    for i in 0..archive.len() {
        // Extract all necessary information from the ZipFile before any await
        let file_path: PathBuf;
        let is_directory: bool;
        let mut file_data: Vec<u8> = Vec::new();
        
        {
            let mut file = archive.by_index(i)
                .map_err(|e| ProcessorError::Processing(format!("Failed to access file in archive: {}", e)))?;
            
            file_path = extract_path.join(file.mangled_name());
            is_directory = file.name().ends_with('/');
            
            // If it's a file, read its data
            if !is_directory {
                use std::io::Read;
                file.read_to_end(&mut file_data)
                    .map_err(|e| ProcessorError::Processing(format!("Failed to read file data: {}", e)))?;
            }
        } // file is dropped here

        // Now we can safely await because the ZipFile is no longer in scope
        if is_directory {
            fs::create_dir_all(&file_path).await
                .map_err(|e| ProcessorError::Processing(format!("Failed to create directory: {}", e)))?;
        } else {
            // Ensure the parent directory exists
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| ProcessorError::Processing(format!("Failed to create parent directory: {}", e)))?;
            }
            
            // Write the file data
            fs::write(&file_path, &file_data).await
                .map_err(|e| ProcessorError::Processing(format!("Failed to write file: {}", e)))?;
        }
        
        if let Some(progress) = pb {
            progress.inc(1);
        }
    }
    
    if let Some(progress) = pb {
        progress.set_message("Archive extracted successfully");
    }
    
    Ok(())
}

/// Saves byte data to a file
pub async fn save_file(output_path: &Path, bytes: Vec<u8>) -> Result<()> {
    fs::write(output_path, bytes)
        .await
        .map_err(|e| ProcessorError::IO(e))
}

async fn some_function() -> Result<()> {
    let _result = fs::read_to_string("some_file.txt")
        .await
        .map_err(|e| ProcessorError::IO(e))?;
    Ok(())
}

async fn another_function() -> Result<()> {
    let response = reqwest::Client::new().get("https://example.com")
        .send()
        .await
        .map_err(|e| ProcessorError::Http(e))?;
    
    let _json = response.json::<serde_json::Value>()
        .await
        .map_err(|e| ProcessorError::Http(e))?;
    
    Ok(())
}

/// Extracts an archive asynchronously with optional progress reporting
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
        // Extract all necessary information from the ZipFile before any await
        let file_path: PathBuf;
        let is_directory: bool;
        let mut file_data: Vec<u8> = Vec::new();
        
        {
            let mut file = archive.by_index(i)
                .map_err(|e| ProcessorError::Processing(format!("Failed to access file in archive: {}", e)))?;
            
            file_path = extract_path.join(file.mangled_name());
            is_directory = file.name().ends_with('/');
            
            // If it's a file, read its data
            if !is_directory {
                use std::io::Read;
                file.read_to_end(&mut file_data)
                    .map_err(|e| ProcessorError::Processing(format!("Failed to read file data: {}", e)))?;
            }
        } // file is dropped here

        // Now we can safely await because the ZipFile is no longer in scope
        if is_directory {
            fs::create_dir_all(&file_path).await
                .map_err(|e| ProcessorError::Processing(format!("Failed to create directory: {}", e)))?;
        } else {
            // Ensure the parent directory exists
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| ProcessorError::Processing(format!("Failed to create parent directory: {}", e)))?;
            }
            
            // Write the file data
            fs::write(&file_path, &file_data).await
                .map_err(|e| ProcessorError::Processing(format!("Failed to write file: {}", e)))?;
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

/// Creates an HTTP client with a custom user agent header
///
/// This function configures a reqwest HTTP client with the "llama-package-service"
/// user agent and a 30-second timeout. This helps ensure that requests from this
/// application are identifiable by servers.
///
/// # Returns
/// A configured reqwest Client ready to make HTTP requests
pub fn create_client_with_user_agent() -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("llama-package-service"),
    );
    
    Client::builder()
        .timeout(Duration::from_secs(30))
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Creates a standardized progress bar for all processors
pub fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    setup_progress_style(&pb);
    pb
}

/// Common utility function to save processor output in multiple formats
pub async fn save_processor_output(
    package_name: &str,
    package_type: &str,
    content: &str,
    output_dir: &Path,
    pb: &ProgressBar
) -> Result<PathBuf> {
    // Create timestamp for filename
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    
    // Create specific directory if it doesn't exist
    let type_dir = match package_type {
        "github_repo" => "github_repos",
        "github_org" => "github_orgs",
        "pypi" => "pypi_packages",
        "pypi_profile" => "pypi_profiles",
        "npm" => "npm_packages",
        "crate" => "rust_crates",
        "go" => "go_packages",
        _ => "other",
    };
    
    let target_dir = output_dir.join(type_dir);
    fs::create_dir_all(&target_dir).await
        .map_err(|e| ProcessorError::IO(e))?;
    
    // Save markdown version
    let md_filename = format!("{}.md", package_name);
    let md_path = target_dir.join(&md_filename);
    fs::write(&md_path, content).await
        .map_err(|e| ProcessorError::IO(e))?;
    
    // Save txt version with timestamp for better organization
    let txt_filename = format!("{}_{}_{}_{}.txt", timestamp, package_name, package_type, "processed");
    let txt_path = target_dir.join(&txt_filename);
    fs::write(&txt_path, content).await
        .map_err(|e| ProcessorError::IO(e))?;
    
    pb.set_message(format!("[SAVED] Output to {} and {}", md_filename, txt_filename));
    
    // Return the markdown path as primary output
    Ok(md_path)
}

/// Generate a standardized package information section for markdown output
pub fn generate_package_info_section(
    package_name: &str,
    package_type: &str,
    info: &[(&str, &str)]
) -> String {
    let mut content = String::new();
    
    // Add header
    let title = match package_type {
        "github_repo" => "GitHub Repository",
        "github_org" => "GitHub Organization",
        "pypi" => "PyPI Package",
        "pypi_profile" => "PyPI User Profile",
        "npm" => "NPM Package",
        "crate" => "Rust Crate",
        "go" => "Go Package",
        _ => "Package",
    };
    
    writeln!(content, "# {}: {}", title, package_name).unwrap();
    writeln!(content).unwrap();
    
    // Add package information section
    writeln!(content, "## Package Information").unwrap();
    writeln!(content).unwrap();
    
    for (key, value) in info {
        writeln!(content, "- **{}**: {}", key, value).unwrap();
    }
    
    // Add processing timestamp
    writeln!(content).unwrap();
    writeln!(content, "## Processed").unwrap();
    writeln!(content).unwrap();
    writeln!(content, "- **Timestamp**: {}", Utc::now().to_rfc3339()).unwrap();
    
    content
}

/// Download a package from its URL and save it to the specified path
pub async fn download_package(url: &str, output_path: &Path, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Downloading package from {}", url));
    
    let client = create_client_with_user_agent();
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| ProcessorError::Http(e))?;
    
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to download package: HTTP {}", 
            response.status()
        )));
    }
    
    let content = response.bytes()
        .await
        .map_err(|e| ProcessorError::Http(e))?;
    
    fs::write(output_path, content).await
        .map_err(|e| ProcessorError::IO(e))?;
    
    pb.set_message(format!("[DOWNLOADED] Package to {}", output_path.display()));
    Ok(())
}

/// Extract common package information from JSON response
pub fn extract_package_info(json: &Value) -> Vec<(&'static str, String)> {
    let mut info = Vec::new();
    
    // Try to extract common fields
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        info.push(("Name", name.to_string()));
    }
    
    if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
        info.push(("Version", version.to_string()));
    }
    
    if let Some(description) = json.get("description").and_then(|v| v.as_str()) {
        info.push(("Description", description.to_string()));
    }
    
    if let Some(author) = json.get("author").and_then(|v| v.as_str()) {
        info.push(("Author", author.to_string()));
    } else if let Some(author) = json.get("author").and_then(|v| v.as_object()).and_then(|o| o.get("name")).and_then(|v| v.as_str()) {
        info.push(("Author", author.to_string()));
    }
    
    if let Some(license) = json.get("license").and_then(|v| v.as_str()) {
        info.push(("License", license.to_string()));
    }
    
    if let Some(homepage) = json.get("homepage").and_then(|v| v.as_str()) {
        info.push(("Homepage", homepage.to_string()));
    }
    
    if let Some(repository) = json.get("repository").and_then(|v| v.as_str()) {
        info.push(("Repository", repository.to_string()));
    } else if let Some(repository) = json.get("repository").and_then(|v| v.as_object()).and_then(|o| o.get("url")).and_then(|v| v.as_str()) {
        info.push(("Repository", repository.to_string()));
    }
    
    info
}

/// Saves processor output in multiple formats (txt and md) with organized structure
///
/// # Arguments
/// * `package_name` - Name of the package being processed
/// * `package_type` - Type of package (github, pypi, npm, crate, go)
/// * `content` - The content to save
/// * `output_dir` - Directory to save output to
/// * `pb` - Progress bar for displaying status
///
/// # Returns
/// Path to the saved primary output file
pub async fn save_comprehensive_output(
    package_name: &str,
    package_type: &str,
    content: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<PathBuf> {
    // Create timestamp for unique filenames
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Create sanitized package name (remove invalid filename chars)
    let sanitized_name = sanitize_filename(package_name);
    
    // Determine package directory based on type
    let type_dir = match package_type {
        "github" => "github_repos",
        "github_org" => "github_orgs",
        "pypi" => "pypi_packages",
        "pypi_profile" => "pypi_profiles",
        "npm" => "npm_packages",
        "crate" => "rust_crates",
        "go" => "go_packages",
        _ => "other",
    };
    
    // Create the type directory if it doesn't exist
    let target_dir = output_dir.join(type_dir);
    tokio::fs::create_dir_all(&target_dir).await?;
    
    // Generate filenames
    let txt_filename = format!("{}_{}_{}_{}.txt", timestamp, sanitized_name, package_type, "processed");
    let md_filename = format!("{}.md", sanitized_name);
    
    // Create paths
    let txt_path = target_dir.join(&txt_filename);
    let md_path = target_dir.join(&md_filename);
    
    // Also save a standardized version to the root output directory if it's a GitHub repo
    let _root_output_path = if package_type == "github" {
        let root_filename = format!("{}_github_repo.txt", sanitized_name);
        let path = output_dir.join(&root_filename);
        tokio::fs::write(&path, content).await?;
        Some(path)
    } else {
        None
    };
    
    // Save both formats
    pb.set_message(format!("Saving {} documentation...", package_name));
    
    tokio::fs::write(&txt_path, content).await?;
    tokio::fs::write(&md_path, content).await?;
    
    // Return the path to the primary output file (txt)
    Ok(txt_path)
}

/// Organize and clean the content for better readability
///
/// # Arguments
/// * `content` - Raw content to be organized
/// * `package_name` - Name of the package for customizing the organization
/// * `package_type` - Type of package (github, pypi, npm, crate, go)
///
/// # Returns
/// Organized and cleaned content
pub fn organize_content(content: &str, package_name: &str, package_type: &str) -> String {
    let mut organized = String::new();
    
    // Add title and metadata
    organized.push_str(&format!("# {} Documentation\n\n", package_name));
    organized.push_str(&format!("**Type:** {}\n", package_type.to_uppercase()));
    organized.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    
    // Add table of contents
    organized.push_str("## Table of Contents\n\n");
    
    // Depending on content and package type, generate appropriate TOC
    match package_type {
        "github" => {
            organized.push_str("1. [Overview](#overview)\n");
            organized.push_str("2. [Repository Information](#repository-information)\n");
            organized.push_str("3. [Code Structure](#code-structure)\n");
            organized.push_str("4. [Key Files](#key-files)\n");
            organized.push_str("5. [Dependencies](#dependencies)\n");
            organized.push_str("6. [Usage Examples](#usage-examples)\n");
        },
        "pypi" => {
            organized.push_str("1. [Package Information](#package-information)\n");
            organized.push_str("2. [Installation](#installation)\n");
            organized.push_str("3. [Dependencies](#dependencies)\n");
            organized.push_str("4. [API Documentation](#api-documentation)\n");
            organized.push_str("5. [Usage Examples](#usage-examples)\n");
        },
        "npm" => {
            organized.push_str("1. [Package Information](#package-information)\n");
            organized.push_str("2. [Installation](#installation)\n");
            organized.push_str("3. [Dependencies](#dependencies)\n");
            organized.push_str("4. [API Documentation](#api-documentation)\n");
            organized.push_str("5. [Usage Examples](#usage-examples)\n");
        },
        "crate" => {
            organized.push_str("1. [Crate Information](#crate-information)\n");
            organized.push_str("2. [Installation](#installation)\n");
            organized.push_str("3. [Dependencies](#dependencies)\n");
            organized.push_str("4. [API Documentation](#api-documentation)\n");
            organized.push_str("5. [Usage Examples](#usage-examples)\n");
        },
        "go" => {
            organized.push_str("1. [Package Information](#package-information)\n");
            organized.push_str("2. [Installation](#installation)\n");
            organized.push_str("3. [Dependencies](#dependencies)\n");
            organized.push_str("4. [API Documentation](#api-documentation)\n");
            organized.push_str("5. [Usage Examples](#usage-examples)\n");
        },
        _ => {
            organized.push_str("1. [Overview](#overview)\n");
            organized.push_str("2. [Documentation](#documentation)\n");
        }
    }
    organized.push_str("\n---\n\n");
    
    // Append the original content
    organized.push_str(content);
    
    // Add footer
    organized.push_str("\n\n---\n\n");
    organized.push_str(&format!("*Documentation generated by LlamaPackageService for {} ({})*\n", package_name, package_type));
    
    organized
}

/// Sanitize a string for use as a filename
///
/// # Arguments
/// * `name` - The name to sanitize
///
/// # Returns
/// A sanitized string valid for filenames
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c
        })
        .collect()
}