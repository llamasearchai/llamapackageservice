use crate::error::{ProcessorError, Result};
use crate::config::Config;
use crate::processors::common::{self, download_file, setup_progress_style};
use crate::processors::PackageProcessor;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use indicatif::ProgressBar;
use reqwest::Client;
use serde_json::Value;
use tokio::fs;
use regex::Regex;
use std::collections::HashMap;
use chrono::Utc;
use std::cell::RefCell;
use url::Url;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tokio::io::AsyncWriteExt;
use std::io::{self, Write};
use tempfile::TempDir;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use zip::ZipArchive;
use tempfile::tempdir;

const PYPI_API_BASE: &str = "https://pypi.org/pypi";
const GITHUB_API_BASE: &str = "https://api.github.com";

// Add a flag to track processing state and avoid recursion
thread_local! {
    static PROCESSING: RefCell<bool> = RefCell::new(false);
}

/// Processor for handling PyPI (Python Package Index) packages
///
/// This processor can extract information from PyPI package URLs or pip install
/// commands, download the packages, and generate comprehensive documentation.
pub struct PyPiProcessor {
    /// HTTP client for making requests to the PyPI API
    client: Client,
}

impl PyPiProcessor {
    /// Creates a new PyPI processor with an extended timeout for handling large packages
    ///
    /// # Returns
    /// A new PyPiProcessor instance configured for PyPI package processing
    pub fn new() -> Self {
        println!("Creating new PyPiProcessor");
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120)) // Increase timeout for large packages
                .build()
                .unwrap_or_else(|_| Client::new())
        }
    }

    async fn validate_pypi_package(&self, package_name: &str) -> Result<()> {
        println!("Validating PyPI package: {}", package_name);
        let url = format!("{}/{}/json", PYPI_API_BASE, package_name);
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Validation(format!("Invalid PyPI package: {}", package_name)));
        }
        
        Ok(())
    }
    
    async fn extract_package_name(&self, url: &str) -> Result<String> {
        println!("Extracting package name from: {}", url);
        // Handle various PyPI URL formats
        // 1. https://pypi.org/project/package-name/
        if let Some(name) = self.extract_from_pypi_url(url) {
            return Ok(name);
        }
        
        // 2. pip install package-name
        if let Some(name) = self.extract_from_pip_command(url) {
            return Ok(name);
        }
        
        // 3. package-name (direct package name)
        if !url.contains('/') && !url.contains(' ') {
            return Ok(url.to_string());
        }
        
        Err(ProcessorError::Validation(format!("Could not extract PyPI package name from: {}", url)))
    }
    
    fn extract_from_pypi_url(&self, url: &str) -> Option<String> {
        let re = Regex::new(r"https?://pypi\.org/project/([^/]+)").ok()?;
        re.captures(url)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }
    
    fn extract_from_pip_command(&self, command: &str) -> Option<String> {
        let re = Regex::new(r"pip\s+install\s+([^\s]+)").ok()?;
        re.captures(command)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }
    
    async fn get_package_info(&self, package_name: &str) -> Result<Value> {
        let url = format!("{}/{}/json", PYPI_API_BASE, package_name);
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| ProcessorError::Http(e))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Processing(format!("Failed to fetch PyPI package info: HTTP {}", response.status())));
        }
        
        let json = response.json::<Value>()
            .await
            .map_err(|e| ProcessorError::Http(e))?;
        
        Ok(json)
    }

    /// Download the package source distribution or wheel
    async fn download_package(&self, package_info: &Value, temp_dir: &Path) -> Result<PathBuf> {
        let releases = &package_info["releases"];
        let version = package_info["info"]["version"].as_str()
            .ok_or_else(|| ProcessorError::Processing("Unable to determine package version".into()))?;
        
        let release = &releases[version];
        
        if !release.is_array() || release.as_array().unwrap().is_empty() {
            return Err(ProcessorError::Processing(format!("No release files found for version {}", version)));
        }

        // Prefer source distribution (sdist) over wheel
        let mut sdist = None;
        let mut wheel = None;

        for file in release.as_array().unwrap() {
            let package_type = file["packagetype"].as_str().unwrap_or("");
            
            if package_type == "sdist" {
                sdist = Some(file);
            } else if package_type.contains("wheel") && wheel.is_none() {
                wheel = Some(file);
            }
        }

        let download_file = sdist.or(wheel).ok_or_else(|| {
            ProcessorError::Processing("No suitable distribution file found".into())
        })?;

        let download_url = download_file["url"].as_str()
            .ok_or_else(|| ProcessorError::Processing("Missing download URL".into()))?;
        
        let filename = download_file["filename"].as_str()
            .ok_or_else(|| ProcessorError::Processing("Missing filename".into()))?;
        
        let output_path = temp_dir.join(filename);
        
        // Download the file
        println!("Downloading package from {}", download_url);
        let response = self.client.get(download_url)
            .send()
            .await
            .map_err(|e| ProcessorError::Http(e))?;
        
        if !response.status().is_success() {
            return Err(ProcessorError::Processing(format!("Failed to download package: HTTP {}", response.status())));
        }
        
        let content = response.bytes()
            .await
            .map_err(|e| ProcessorError::Http(e))?;
        
        tokio::fs::write(&output_path, content).await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(output_path)
    }

    /// Extract the downloaded package
    async fn extract_package(&self, package_path: &Path, extract_dir: &Path) -> Result<()> {
        println!("Extracting package: {}", package_path.display());
        
        let package_path_str = package_path.to_string_lossy().to_string();
        
        if package_path_str.ends_with(".tar.gz") || package_path_str.ends_with(".tgz") {
            // Create extract directory if it doesn't exist
            tokio::fs::create_dir_all(extract_dir).await
                .map_err(|e| ProcessorError::IO(e))?;

            let status = TokioCommand::new("tar")
                .arg("-xzf")
                .arg(&package_path_str)
                .arg("-C")
                .arg(extract_dir)
                .status()
                .await
                .map_err(|e| ProcessorError::IO(e))?;
            
            if !status.success() {
                return Err(ProcessorError::Processing(format!(
                    "Failed to extract tar.gz package: exit code {:?}", status.code()
                )));
            }
        } else if package_path_str.ends_with(".zip") || package_path_str.ends_with(".whl") {
            // Create extract directory if it doesn't exist
            tokio::fs::create_dir_all(extract_dir).await
                .map_err(|e| ProcessorError::IO(e))?;
                
            // Use unzip command for zip files
            let status = TokioCommand::new("unzip")
                .arg("-q") // quiet mode
                .arg("-o") // overwrite files
                .arg(&package_path_str)
                .arg("-d") // destination directory
                .arg(extract_dir)
                .status()
                .await
                .map_err(|e| ProcessorError::IO(e))?;
            
            if !status.success() {
                return Err(ProcessorError::Processing(format!(
                    "Failed to extract zip package: exit code {:?}", status.code()
                )));
            }
        } else {
            return Err(ProcessorError::Processing(format!(
                "Unsupported package format: {}", package_path_str
            )));
        }
        
        Ok(())
    }

    /// Process the extracted package content
    async fn process_package_content(&self, extract_dir: &Path) -> Result<String> {
        println!("Processing package content in {}", extract_dir.display());
        
        // Find all Python files in the extracted directory
        let output = TokioCommand::new("find")
            .arg(extract_dir)
            .arg("-type")
            .arg("f")
            .arg("-name")
            .arg("*.py")
            .output()
            .await
            .map_err(|e| ProcessorError::Processing(format!("Failed to run find command: {}", e)))?;
            
        if !output.status.success() {
            return Err(ProcessorError::Processing("find command failed".into()));
        }
        
        let files = String::from_utf8(output.stdout)
            .map_err(|e| ProcessorError::Processing(format!("Invalid UTF-8 in find output: {}", e)))?;
            
        let mut content = String::new();
        
        // Process each Python file
        for file_path in files.lines() {
            let path = Path::new(file_path);
            
            // Get relative path from extract_dir
            let rel_path = path.strip_prefix(extract_dir)
                .map_err(|e| ProcessorError::Processing(format!("Path stripping failed: {}", e)))?;
                
            // Add file header
            content.push_str(&format!("\n\n## File: {}\n```python\n", 
                rel_path.display()));
            
            if let Ok(file_content) = tokio::fs::read_to_string(path).await {
                content.push_str(&file_content);
                content.push_str("\n```\n\n");
            } else {
                content.push_str("Could not read file content.\n");
            }
        }
        
        // Also look for important metadata files
        for filename in &["README.md", "README.rst", "README.txt", "setup.py", "pyproject.toml", "setup.cfg"] {
            // Use glob to find files case-insensitively
            let output = TokioCommand::new("find")
                .arg(extract_dir)
                .arg("-type")
                .arg("f")
                .arg("-iname")
                .arg(filename)
                .output()
                .await
                .map_err(|e| ProcessorError::Processing(format!("Failed to run find command: {}", e)))?;
                
            if !output.status.success() {
                continue;
            }
            
            let files = String::from_utf8(output.stdout)
                .map_err(|e| ProcessorError::Processing(format!("Invalid UTF-8 in find output: {}", e)))?;
                
            for file_path in files.lines() {
                if file_path.is_empty() {
                    continue;
                }
                
                let path = Path::new(file_path);
                
                // Get relative path from extract_dir
                let rel_path = path.strip_prefix(extract_dir)
                    .map_err(|e| ProcessorError::Processing(format!("Path stripping failed: {}", e)))?;
                    
                // Add metadata file header
                content.push_str(&format!("\n\n## Metadata File: {}\n```\n", 
                    rel_path.display()));
                
                if let Ok(file_content) = tokio::fs::read_to_string(path).await {
                    content.push_str(&file_content);
                    content.push_str("\n```\n\n");
                } else {
                    content.push_str("Could not read file content.\n");
                }
            }
        }
        
        Ok(content)
    }

    /// Get the repository URL from package info
    fn get_repository_url(&self, package_info: &Value) -> Option<String> {
        // Check project_urls for repository information
        if let Some(urls) = package_info["info"]["project_urls"].as_object() {
            for (key, url) in urls {
                if key.to_lowercase().contains("repo") || 
                   key.to_lowercase().contains("source") || 
                   key.to_lowercase().contains("github") ||
                   key.to_lowercase().contains("gitlab") ||
                   key.to_lowercase().contains("bitbucket") {
                    if let Some(url_str) = url.as_str() {
                        return Some(url_str.to_string());
                    }
                }
            }
        }
        
        // Check home_page as fallback
        if let Some(homepage) = package_info["info"]["home_page"].as_str() {
            if homepage.contains("github.com") || 
               homepage.contains("gitlab.com") || 
               homepage.contains("bitbucket.org") {
                return Some(homepage.to_string());
            }
        }
        
        None
    }

    async fn process_package(&self, package_name: &str, output_dir: &Path) -> Result<()> {
        println!("Processing package: {}", package_name);
        
        // Get package information
        let package_info = self.get_package_info(package_name).await?;
        
        // Create temporary directory for package extraction
        let temp_dir = TempDir::new()
            .map_err(|e| ProcessorError::IO(io::Error::new(io::ErrorKind::Other, e)))?;
        let extract_dir = TempDir::new()
            .map_err(|e| ProcessorError::IO(io::Error::new(io::ErrorKind::Other, e)))?;
        
        // Download and extract package
        let package_path = self.download_package(&package_info, temp_dir.path()).await?;
        self.extract_package(&package_path, extract_dir.path()).await?;
        
        // Process package content
        let package_content = self.process_package_content(extract_dir.path()).await?;
        
        // Extract relevant information from package_info
        let info = &package_info["info"];
        let version = info["version"].as_str().unwrap_or("unknown");
        let summary = info["summary"].as_str().unwrap_or("");
        let description = info["description"].as_str().unwrap_or("");
        let author = info["author"].as_str().unwrap_or("Unknown");
        let author_email = info["author_email"].as_str().unwrap_or("");
        let license = info["license"].as_str().unwrap_or("Unknown");
        let homepage = info["home_page"].as_str().unwrap_or("");
        
        // Get repository URL if available
        let repository_url = self.get_repository_url(&package_info).unwrap_or_default();
        
        // Create output content
        let mut content = String::new();
        content.push_str(&format!("# PyPI Package: {}\n\n", package_name));
        content.push_str(&format!("Version: {}\n", version));
        content.push_str(&format!("Summary: {}\n", summary));
        content.push_str(&format!("Author: {}", author));
        if !author_email.is_empty() {
            content.push_str(&format!(" ({})", author_email));
        }
        content.push_str("\n");
        content.push_str(&format!("License: {}\n", license));
        if !homepage.is_empty() {
            content.push_str(&format!("Project URL: {}\n", homepage));
        }
        if !repository_url.is_empty() {
            content.push_str(&format!("Repository: {}\n", repository_url));
        }
        
        content.push_str("\n## Description\n\n");
        content.push_str(description);
        
        // Add package dependencies
        if let Some(requires_dist) = package_info["info"]["requires_dist"].as_array() {
            content.push_str("\n\n## Dependencies\n\n");
            for req in requires_dist {
                if let Some(req_str) = req.as_str() {
                    content.push_str(&format!("- {}\n", req_str));
                }
            }
        }
        
        // Add package classifiers
        if let Some(classifiers) = package_info["info"]["classifiers"].as_array() {
            content.push_str("\n\n## Classifiers\n\n");
            for classifier in classifiers {
                if let Some(class_str) = classifier.as_str() {
                    content.push_str(&format!("- {}\n", class_str));
                }
            }
        }
        
        // Add package content
        content.push_str("\n\n## Package Content\n");
        content.push_str(&package_content);
        
        // Save to file with a well-structured, organized naming convention
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let sanitized_name = package_name.replace('/', "_").replace('\\', "_");
        let filename = format!("{}_{}_pypi.txt", timestamp, sanitized_name);
        let output_path = output_dir.join(filename);
        
        fs::write(&output_path, content).await?;
        
        println!("[SUCCESS] Package processed successfully. Output saved to: {}", output_path.display());
        
        Ok(())
    }
}

#[async_trait]
impl PackageProcessor for PyPiProcessor {
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        common::setup_progress_style(&pb);
        
        // Extract package name
        let package_name = self.extract_package_name(url).await?;
        
        // Process directly
        process_pypi_package_thoroughly(&package_name, output_dir, &pb).await?;
        
        pb.finish_with_message(format!("✨ PyPI package {} processed successfully", package_name));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "PyPI"
    }

    fn accepts(&self, url: &str) -> bool {
        url.contains("pypi.org") || 
        url.contains("pip install") || 
        (!url.contains('/') && !url.contains(' ') && !url.contains('.'))
    }

    async fn validate(&self, url: &str) -> Result<()> {
        let package_name = self.extract_package_name(url).await?;
        self.validate_pypi_package(&package_name).await
    }
}

impl Clone for PyPiProcessor {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone()
        }
    }
}

/// Process a PyPI package with thorough extraction and documentation
///
/// # Arguments
/// * `package_name` - Name of the PyPI package
/// * `output_dir` - Directory to save output to
/// * `pb` - Progress bar for displaying status
///
/// # Returns
/// Result indicating success or failure
pub async fn process_pypi_package_thoroughly(package_name: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Processing PyPI package: {}", package_name));
    
    // Create client
    let client = Arc::new(Client::builder()
        .timeout(Duration::from_secs(300))  // Increased timeout for large packages
        .build()
        .unwrap_or_default());
    
    // Get package info
    pb.set_message(format!("Fetching package info for: {}", package_name));
    let url = format!("https://pypi.org/pypi/{}/json", package_name);
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to fetch package info: HTTP {}", 
            response.status()
        )));
    }
    
    let package_info: serde_json::Value = response.json().await?;
    
    // Create temp directory
    let temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;
    
    // Download the package
    pb.set_message(format!("Downloading package: {}", package_name));
    let download_url = if let Some(url) = package_info["urls"].as_array()
        .and_then(|urls| urls.iter()
            .filter(|u| u["packagetype"].as_str().map_or(false, |t| t == "sdist"))
            .next()
            .or_else(|| urls.first())
            .and_then(|u| u["url"].as_str())
        ) {
        url.to_string()
    } else {
        return Err(ProcessorError::Validation("Could not find download URL".to_string()));
    };
    
    pb.set_message(format!("Downloading package from {}", download_url));
    
    // Determine file extension based on download URL
    let file_extension = if download_url.ends_with(".tar.gz") || download_url.ends_with(".tgz") {
        ".tar.gz"
    } else if download_url.ends_with(".zip") || download_url.ends_with(".whl") {
        ".zip"
    } else {
        // Default to tar.gz if can't determine
        ".tar.gz"
    };
    
    let package_path = temp_dir.path().join(format!("{}{}", package_name, file_extension));
    
    // Download package
    let response = client.get(&download_url).send().await?;
    let bytes = response.bytes().await?;
    tokio::fs::write(&package_path, &bytes).await?;
    
    // Extract the package
    pb.set_message(format!("Extracting package: {}", package_name));
    let extract_dir = temp_dir.path().join("extract");
    tokio::fs::create_dir_all(&extract_dir).await?;
    
    // Use system commands for extraction which are more reliable
    if file_extension == ".tar.gz" {
        // Use tar command for tar.gz files
        let status = tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(&package_path)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        if !status.success() {
            return Err(ProcessorError::Processing(format!(
                "Failed to extract tar.gz package: exit code {:?}", status.code()
            )));
        }
    } else if file_extension == ".zip" {
        // Use unzip command for zip files
        let status = tokio::process::Command::new("unzip")
            .arg("-q") // quiet mode
            .arg("-o") // overwrite files
            .arg(&package_path)
            .arg("-d") // destination directory
            .arg(&extract_dir)
            .status()
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        if !status.success() {
            return Err(ProcessorError::Processing(format!(
                "Failed to extract zip package: exit code {:?}", status.code()
            )));
        }
    } else {
        return Err(ProcessorError::Processing("Unsupported archive format".to_string()));
    }
    
    // Start building comprehensive output
    let mut content = String::new();
    
    // Header
    content.push_str(&format!("# {} Documentation\n\n", package_name));
    content.push_str("**Type:** PYPI\n");
    content.push_str(&format!("**Generated:** {} UTC\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
    
    // Table of Contents
    content.push_str("## Table of Contents\n\n");
    content.push_str("1. [Package Information](#package-information)\n");
    content.push_str("2. [Installation](#installation)\n");
    content.push_str("3. [Dependencies](#dependencies)\n");
    content.push_str("4. [API Documentation](#api-documentation)\n");
    content.push_str("5. [Usage Examples](#usage-examples)\n");
    content.push_str("6. [Complete Source Code](#complete-source-code)\n\n");
    content.push_str("---\n\n");
    
    // Package Information Section
    content.push_str("## Package Information\n\n");
    
    // Add basic info
    let version = package_info["info"]["version"].as_str().unwrap_or("unknown");
    let description = package_info["info"]["description"].as_str().unwrap_or("No description available");
    let author = package_info["info"]["author"].as_str().unwrap_or("Unknown");
    let author_email = package_info["info"]["author_email"].as_str().unwrap_or("Not specified");
    let home_page = package_info["info"]["home_page"].as_str().unwrap_or("Not specified");
    let project_urls = &package_info["info"]["project_urls"];
    
    content.push_str(&format!("- **Name**: {}\n", package_name));
    content.push_str(&format!("- **Version**: {}\n", version));
    content.push_str(&format!("- **Author**: {}\n", author));
    content.push_str(&format!("- **Author Email**: {}\n", author_email));
    content.push_str(&format!("- **Home Page**: {}\n", home_page));
    
    // Project URLs
    content.push_str("\n### Project URLs\n\n");
    if let Some(urls) = project_urls.as_object() {
        if urls.is_empty() {
            content.push_str("No project URLs specified.\n");
        } else {
            for (key, value) in urls {
                content.push_str(&format!("- **{}**: {}\n", key, value.as_str().unwrap_or("Not specified")));
            }
        }
    } else {
        content.push_str("No project URLs specified.\n");
    }
    
    // Installation Section
    content.push_str("\n## Installation\n\n");
    content.push_str("Install this package using pip:\n\n");
    content.push_str("```bash\n");
    content.push_str(&format!("pip install {}\n", package_name));
    content.push_str("```\n\n");
    
    // Dependencies Section
    content.push_str("## Dependencies\n\n");
    if let Some(requires_dist) = package_info["info"]["requires_dist"].as_array() {
        if requires_dist.is_empty() {
            content.push_str("This package has no dependencies.\n");
        } else {
            content.push_str("This package depends on the following packages:\n\n");
            for dep in requires_dist {
                content.push_str(&format!("- {}\n", dep.as_str().unwrap_or("Unknown dependency")));
            }
        }
    } else {
        content.push_str("No dependencies information available.\n");
    }
    
    // Description
    content.push_str("\n## Description\n\n");
    content.push_str("### Overview\n");
    if let Some(summary) = package_info["info"]["summary"].as_str() {
        if !summary.is_empty() {
            content.push_str(&format!("{}\n\n", summary));
        }
    }
    content.push_str(&format!("{}\n\n", description));
    
    // API Documentation Section
    pb.set_message("Processing API documentation...");
    content.push_str("## API Documentation\n\n");
    
    // Find all Python files for API documentation
    let mut python_files = Vec::new();
    find_python_files(&extract_dir, &mut python_files).await?;
    
    if python_files.is_empty() {
        content.push_str("No Python source files found for API documentation extraction.\n");
    } else {
        python_files.sort();
        
        for file_path in &python_files {
            let rel_path = file_path.strip_prefix(&extract_dir)
                .unwrap_or(file_path)
                .to_string_lossy();
            
            content.push_str(&format!("\n### File: `{}`\n\n", rel_path));
            
            if let Ok(file_content) = tokio::fs::read_to_string(file_path).await {
                let api_docs = extract_python_api_docs(&file_content);
                content.push_str(&api_docs);
            } else {
                content.push_str("No API documentation found in this file.\n");
            }
        }
    }
    
    // Usage Examples Section
    content.push_str("\n## Usage Examples\n\n");
    
    // Look for README files
    let readme_path = find_readme(&extract_dir).await;
    if let Some(path) = readme_path {
        if let Ok(readme_content) = tokio::fs::read_to_string(&path).await {
            content.push_str("### From README\n\n");
            content.push_str(&extract_code_blocks(&readme_content));
        }
    }
    
    // Look for example files
    let mut example_files = Vec::new();
    find_example_files(&extract_dir, &mut example_files).await?;
    
    if !example_files.is_empty() {
        content.push_str("\n### From Examples\n\n");
        example_files.sort();
        
        for file_path in &example_files {
            let rel_path = file_path.strip_prefix(&extract_dir)
                .unwrap_or(file_path)
                .to_string_lossy();
            
            content.push_str(&format!("#### Example: `{}`\n\n", rel_path));
            
            if let Ok(example_content) = tokio::fs::read_to_string(file_path).await {
                content.push_str("```python\n");
                content.push_str(&example_content);
                content.push_str("\n```\n\n");
            }
        }
    } else {
        content.push_str("No example files found.\n");
    }
    
    // Complete Source Code Section - This is the new comprehensive part
    pb.set_message("Processing complete source code...");
    content.push_str("\n## Complete Source Code\n\n");
    
    // Find ALL files in the package
    let mut all_files = Vec::new();
    find_all_files(&extract_dir, &mut all_files).await?;
    
    if all_files.is_empty() {
        content.push_str("No files found in the package.\n");
    } else {
        // Sort files by path for consistent output
        all_files.sort();
        
        // Display package structure first
        content.push_str("### Package Structure\n\n");
        content.push_str("```\n");
        for file_path in &all_files {
            let rel_path = file_path.strip_prefix(&extract_dir)
                .unwrap_or(file_path)
                .to_string_lossy();
            content.push_str(&format!("{}\n", rel_path));
        }
        content.push_str("```\n\n");
        
        // Now include complete source code for all files
        content.push_str("### Complete File Contents\n\n");
        
        for file_path in &all_files {
            let rel_path = file_path.strip_prefix(&extract_dir)
                .unwrap_or(file_path)
                .to_string_lossy();
            
            content.push_str(&format!("#### File: `{}`\n\n", rel_path));
            
            // Determine file type for syntax highlighting
            let file_extension = file_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("txt");
            
            let language = match file_extension {
                "py" => "python",
                "md" => "markdown",
                "rst" => "rst",
                "txt" => "text",
                "toml" => "toml",
                "yaml" | "yml" => "yaml",
                "json" => "json",
                "xml" => "xml",
                "html" => "html",
                "css" => "css",
                "js" => "javascript",
                "sh" => "bash",
                "cfg" | "ini" => "ini",
                _ => "text",
            };
            
            // Read and include the complete file content
            if let Ok(file_content) = tokio::fs::read_to_string(file_path).await {
                content.push_str(&format!("```{}\n", language));
                content.push_str(&file_content);
                content.push_str("\n```\n\n");
            } else {
                // Try to read as binary and show info
                if let Ok(file_metadata) = tokio::fs::metadata(file_path).await {
                    if file_metadata.len() > 0 {
                        content.push_str(&format!("*Binary file ({} bytes)*\n\n", file_metadata.len()));
                    } else {
                        content.push_str("*Empty file*\n\n");
                    }
                } else {
                    content.push_str("*Could not read file*\n\n");
                }
            }
        }
    }
    
    content.push_str("\n---\n\n");
    content.push_str(&format!("*Documentation generated by LlamaPackageService for {} (pypi)*\n", package_name));
    
    // Save comprehensive output
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let sanitized_name = package_name.replace('/', "_").replace('\\', "_");
    let filename = format!("{}_{}_pypi_processed.txt", timestamp, sanitized_name);
    let output_path = output_dir.join(filename);
    
    tokio::fs::write(&output_path, content).await?;
    
    pb.finish_with_message(format!("Package processed successfully. Output saved to: {}", output_path.display()));
    Ok(())
}

/// Find Python files in a directory recursively
async fn find_python_files(dir: &Path, result: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                Box::pin(find_python_files(&path, result)).await?;
            } else if let Some(ext) = path.extension() {
                if ext == "py" {
                    result.push(path);
                }
            }
        }
    }
    Ok(())
}

/// Find all files in a directory recursively
async fn find_all_files(dir: &Path, result: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                Box::pin(find_all_files(&path, result)).await?;
            } else {
                // Include all files, not just Python files
                result.push(path);
            }
        }
    }
    Ok(())
}

/// Find example files in a directory
async fn find_example_files(dir: &Path, result: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                Box::pin(find_example_files(&path, result)).await?;
            } else if let Some(ext) = path.extension() {
                if ext == "py" {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    if file_name.contains("example") || file_name.contains("demo") || file_name.contains("sample") {
                        result.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Find README file in a directory
async fn find_readme(dir: &Path) -> Option<PathBuf> {
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Some(entry) = entries.next_entry().await.ok()? {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();
                    if file_name_str.starts_with("readme") {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Extract code blocks from markdown text
fn extract_code_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut language = "".to_string();
    
    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                in_code_block = false;
                if !code_block_content.is_empty() {
                    result.push_str(&format!("```{}\n", language));
                    result.push_str(&code_block_content);
                    result.push_str("```\n\n");
                    code_block_content.clear();
                    language.clear();
                }
            } else {
                // Start of code block
                in_code_block = true;
                language = line.trim_start_matches('`').to_string();
            }
        } else if in_code_block {
            // Inside code block
            code_block_content.push_str(line);
            code_block_content.push('\n');
        }
    }
    
    if result.is_empty() {
        return "No code examples found in README.\n".to_string();
    }
    
    result
}

/// Extract API documentation from Python code
fn extract_python_api_docs(code: &str) -> String {
    let mut result = String::new();
    let mut current_class = None;
    
    for line in code.lines() {
        let trimmed = line.trim();
        
        // Detect class definitions
        if trimmed.starts_with("class ") && trimmed.contains(':') {
            let class_name = trimmed
                .strip_prefix("class ")
                .unwrap_or(trimmed)
                .split('(')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
                .trim();
            
            if !class_name.is_empty() {
                current_class = Some(class_name.to_string());
                result.push_str(&format!("#### Class: `{}`\n\n", class_name));
            }
        }
        // Detect function definitions
        else if trimmed.starts_with("def ") && trimmed.contains(':') {
            let func_signature = trimmed
                .strip_prefix("def ")
                .unwrap_or(trimmed)
                .split(':')
                .next()
                .unwrap_or("")
                .trim();
            
            if !func_signature.is_empty() {
                if let Some(_class_name) = &current_class {
                    result.push_str(&format!("##### Method: `{}`\n\n", func_signature));
                } else {
                    result.push_str(&format!("#### Function: `{}`\n\n", func_signature));
                }
                
                // Look for docstring
                let mut i = code.lines().position(|l| l == line).unwrap_or(0) + 1;
                let mut docstring = String::new();
                let mut in_docstring = false;
                let mut triple_quote_count = 0;
                
                while i < code.lines().count() {
                    let doc_line = code.lines().nth(i).unwrap_or("").trim_start();
                    
                    if !in_docstring {
                        if doc_line.starts_with("\"\"\"") || doc_line.starts_with("'''") {
                            in_docstring = true;
                            triple_quote_count += 1;
                            
                            // Get remainder of this line if any
                            let remainder = doc_line.get(3..).unwrap_or("").trim();
                            if !remainder.is_empty() {
                                docstring.push_str(remainder);
                                docstring.push('\n');
                            }
                        } else if !doc_line.is_empty() && !doc_line.starts_with("#") {
                            // No docstring found
                            break;
                        }
                    } else {
                        if doc_line.contains("\"\"\"") || doc_line.contains("'''") {
                            triple_quote_count += 1;
                            if triple_quote_count == 2 {
                                // End of docstring
                                break;
                            }
                        }
                        docstring.push_str(doc_line);
                        docstring.push('\n');
                    }
                    
                    i += 1;
                }
                
                if !docstring.is_empty() {
                    result.push_str(&docstring);
                    result.push('\n');
                }
            }
        }
    }
    
    if result.is_empty() {
        return "No API documentation found in this file.\n".to_string();
    }
    
    result
}

/// Select the most important files from a list
fn select_important_files(files: &[PathBuf], max_files: usize) -> Vec<PathBuf> {
    if files.len() <= max_files {
        return files.to_vec();
    }
    
    // Prioritize init files and main module files
    let mut importance_map: Vec<(usize, &PathBuf)> = files.iter().enumerate()
        .map(|(_idx, path)| {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            let path_str = path.to_string_lossy();
            
            let mut score = 0;
            
            // Higher priority for __init__.py files
            if file_name == "__init__.py" {
                score += 100;
            }
            
            // Higher priority for main.py files
            if file_name == "main.py" {
                score += 90;
            }
            
            // Higher priority for files at the root of the package
            if path_str.matches('/').count() < 3 {
                score += 80;
            }
            
            // Higher priority for common module names
            if file_name == "api.py" || file_name == "client.py" || file_name == "core.py" {
                score += 70;
            }
            
            (score, path)
        })
        .collect();
    
    // Sort by importance score (highest first)
    importance_map.sort_by(|a, b| b.0.cmp(&a.0));
    
    // Return top N files
    importance_map.iter()
        .take(max_files)
        .map(|(_, path)| (*path).clone())
        .collect()
}

/// Processes a PyPI package URL, generating comprehensive documentation and analysis
///
/// This function handles the complete processing of a PyPI package, from URL parsing
/// to documentation generation. It provides progress updates via the progress bar.
///
/// # Arguments
/// * `url` - The PyPI package URL or pip install command to process
/// * `output_dir` - The directory where output files will be saved
/// * `pb` - Progress bar for displaying processing status
///
/// # Returns
/// Result indicating success or failure of the processing operation
pub async fn process_pypi_url(url: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    // Check if we're already processing to avoid recursive calls
    let already_processing = PROCESSING.with(|cell| {
        let processing = *cell.borrow();
        if !processing {
            *cell.borrow_mut() = true;
        }
        processing
    });
    
    if already_processing {
        println!("Avoiding recursive call to process_pypi_url");
        return Ok(());
    }
    
    pb.set_message(format!("Processing PyPI package: {}", url));
    
    let processor = PyPiProcessor::new();
    let package_name = processor.extract_package_name(url).await?;
    
    // Use the new comprehensive processing function
    let result = process_pypi_package_thoroughly(&package_name, output_dir, pb).await;
    
    // Reset processing state
    PROCESSING.with(|cell| {
        *cell.borrow_mut() = false;
    });
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_extract_package_name() {
        let processor = PyPiProcessor::new();
        
        assert_eq!(
            processor.extract_package_name("https://pypi.org/project/requests/").await.unwrap(),
            "requests"
        );
        
        assert_eq!(
            processor.extract_package_name("pip install numpy").await.unwrap(),
            "numpy"
        );
        
        assert_eq!(
            processor.extract_package_name("pandas").await.unwrap(),
            "pandas"
        );
    }

    #[tokio::test]
    async fn test_pypi_processor() {
        let temp_dir = TempDir::new().unwrap();
        let processor = PyPiProcessor::new();
        
        // This is a simplified test that only checks the URL extraction
        // In a real test environment, we would mock the PyPI API responses
        let package_name = processor.extract_package_name("requests").await.unwrap();
        assert_eq!(package_name, "requests");
    }
}