use crate::error::{ProcessorError, Result};
use crate::processors::common::{self, setup_progress_style, save_output_file};
use crate::processors::PackageProcessor;
use crate::config::Config;
use async_trait::async_trait;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tokio::fs;
use url::Url;
use std::io::Cursor;
use zip::ZipArchive;
use std::sync::{Arc, Mutex};

const NPM_REGISTRY_API: &str = "https://registry.npmjs.org";

/// NPM package processor for handling NPM package URLs
/// 
/// This processor can extract information from NPM package URLs and download
/// and process the corresponding packages.
#[derive(Debug, Clone)]
pub struct NpmProcessor {
    /// HTTP client for making requests to the NPM registry
    client: Client,
}

impl NpmProcessor {
    /// Creates a new NPM processor with default configuration
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }

    /// Validates if a URL is a valid NPM package URL
    ///
    /// # Arguments
    /// * `url` - The URL to validate
    async fn validate_url(&self, url: &str) -> Result<()> {
        let parsed = Url::parse(url)
            .map_err(|_| ProcessorError::new("Invalid URL format"))?;
        
        if parsed.host_str() != Some("www.npmjs.com") || !parsed.path().starts_with("/package/") {
            return Err(ProcessorError::new("Not a valid NPM package URL"));
        }
        Ok(())
    }

    /// Extracts the package name from an NPM package URL
    ///
    /// # Arguments
    /// * `url` - The NPM package URL to extract the name from
    async fn extract_package_name(&self, url: &str) -> Result<String> {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 5 {
            return Err(ProcessorError::Validation("Invalid NPM URL format".to_string()));
        }
        Ok(parts[4].to_string())
    }

    /// Fetches package information from the NPM registry
    ///
    /// # Arguments
    /// * `package_name` - The name of the NPM package
    async fn get_package_info(&self, package_name: &str) -> Result<serde_json::Value> {
        let url = format!("https://registry.npmjs.org/{}", package_name);
        let response = self.client.get(&url).send().await?;
        let json = response.json().await?;
        Ok(json)
    }

    async fn download_package(&self, package_name: &str, temp_dir: &Path) -> Result<PathBuf> {
        let url = format!("https://registry.npmjs.org/{}/-/{}-latest.tgz", package_name, package_name);
        let response = self.client.get(&url).send().await?;
        let bytes = response.bytes().await?;
        
        let archive_path = temp_dir.join("package.tgz");
        fs::write(&archive_path, bytes).await?;
        Ok(archive_path)
    }

    async fn process_package_contents(&self, package_info: &serde_json::Value, temp_dir: &Path, output_dir: &Path) -> Result<()> {
        let readme = fs::read_to_string(temp_dir.join("README.md")).await
            .unwrap_or_else(|_| String::from("No README found"));

        let package_json = fs::read_to_string(temp_dir.join("package.json")).await
            .unwrap_or_else(|_| String::from("No package.json found"));

        let mut structure = String::new();
        self.build_file_tree(temp_dir, "", &mut structure).await?;

        let content = format!(
            "# {}\n\n## Package Information\n\n```json\n{}\n```\n\n## README\n\n{}\n\n## File Structure\n\n```\n{}\n```\n",
            package_info["name"].as_str().unwrap_or("Unknown"),
            package_json,
            readme,
            structure
        );

        fs::write(output_dir.join("output.md"), content).await?;
        Ok(())
    }

    /// Builds a file tree representation of a directory structure
    ///
    /// # Arguments
    /// * `dir` - The directory to build the tree from
    /// * `prefix` - The prefix to use for indentation (increases with depth)
    /// * `output` - The string to append the tree representation to
    async fn build_file_tree(&self, dir: &Path, prefix: &str, output: &mut String) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();
            
            output.push_str(&format!("{}{}\n", prefix, name));
            
            if path.is_dir() {
                let new_prefix = format!("{}  ", prefix);
                Box::pin(self.build_file_tree(&path, &new_prefix, output)).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl PackageProcessor for NpmProcessor {
    /// Processes an NPM package URL, extracting information and generating documentation
    ///
    /// # Arguments
    /// * `url` - The NPM package URL to process
    /// * `output_dir` - The directory to save output files to
    /// * `_config` - The application configuration
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        setup_progress_style(&pb);
        
        // Extract package name
        let package_name = extract_npm_package_name(url)?;
        
        // Process directly
        process_npm_package_by_name(&package_name, output_dir, &pb).await?;
        
        pb.finish_with_message(format!("✨ NPM package {} processed successfully", package_name));
        Ok(())
    }

    /// Returns the name of this processor
    fn name(&self) -> &'static str {
        "NPM"
    }

    /// Checks if this processor accepts the given URL
    ///
    /// # Arguments
    /// * `url` - The URL to check
    fn accepts(&self, url: &str) -> bool {
        url.contains("npmjs.com") || url.contains("npm install")
    }

    /// Validates if a URL is supported by this processor
    ///
    /// # Arguments
    /// * `url` - The URL to validate
    async fn validate(&self, url: &str) -> Result<()> {
        self.validate_url(url).await
    }
}

/// Extracts an NPM package name from different URL formats
///
/// # Arguments
/// * `url` - The URL or command string to extract the package name from
///
/// # Returns
/// The extracted NPM package name
fn extract_npm_package_name(url: &str) -> Result<String> {
    if url.contains("npmjs.com/package/") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 5 {
            return Err(ProcessorError::Validation("Invalid NPM URL format".to_string()));
        }
        Ok(parts[4].to_string())
    } else if url.contains("npm install ") {
        let parts: Vec<&str> = url.split("npm install ").collect();
        if parts.len() < 2 {
            return Err(ProcessorError::Validation("Invalid NPM command format".to_string()));
        }
        // Take the first word after "npm install"
        let package_parts: Vec<&str> = parts[1].split_whitespace().collect();
        Ok(package_parts[0].to_string())
    } else {
        // Just try to use the URL as-is
        Ok(url.to_string())
    }
}

/// Processes an NPM package from a URL or command string
///
/// # Arguments
/// * `url` - The NPM package URL or command to process
/// * `output_dir` - The directory to save output files to
/// * `pb` - Progress bar for displaying progress
///
/// # Returns
/// Result indicating success or failure
pub async fn process_npm_package(url: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    pb.set_message(format!("Processing NPM package: {}", url));
    
    let package_name = extract_npm_package_name(url)?;
    process_npm_package_by_name(&package_name, output_dir, pb).await?;
    
    Ok(())
}

/// Processes an NPM package by its name
///
/// # Arguments
/// * `package_name` - The name of the NPM package to process
/// * `output_dir` - The directory to save output files to
/// * `pb` - Progress bar for displaying progress
///
/// # Returns
/// Result indicating success or failure
async fn process_npm_package_by_name(package_name: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    // Use the progress bar
    pb.set_message(format!("Fetching NPM package: {}", package_name));
    
    // Create a client to fetch package information
    let client = Arc::new(Client::new());
    
    // Get package information from NPM registry
    let url = format!("{}/{}", NPM_REGISTRY_API, package_name);
    pb.set_message(format!("Fetching package info from NPM registry: {}", url));
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to fetch package info: HTTP {}", 
            response.status()
        )));
    }
    
    let package_info: serde_json::Value = response.json().await?;
    
    // Create temp directory for downloads
    let temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;
    
    // Initialize content string
    let mut content = String::new();
    
    // Package Information section
    content.push_str("## Package Information\n\n");
    
    // Latest version and metadata
    let latest_version = package_info["dist-tags"]["latest"].as_str().unwrap_or("unknown");
    
    // Basic package information
    content.push_str(&format!("- **Name:** {}\n", package_name));
    content.push_str(&format!("- **Latest Version:** {}\n", latest_version));
    
    if let Some(description) = package_info["description"].as_str() {
        content.push_str(&format!("- **Description:** {}\n", description));
    }
    
    // Author info
    match &package_info["author"] {
        serde_json::Value::Object(obj) => {
            if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                content.push_str(&format!("- **Author:** {}", name));
                
                if let Some(email) = obj.get("email").and_then(|e| e.as_str()) {
                    content.push_str(&format!(" <{}>\n", email));
                } else {
                    content.push_str("\n");
                }
            }
        },
        serde_json::Value::String(s) => {
            content.push_str(&format!("- **Author:** {}\n", s));
        },
        _ => content.push_str("- **Author:** Unknown\n"),
    }
    
    // Repository
    content.push_str("- **Repository:** ");
    match &package_info["repository"] {
        serde_json::Value::Object(obj) => {
            if let Some(url) = obj.get("url").and_then(|u| u.as_str()) {
                content.push_str(&format!("{}\n", url));
            } else {
                content.push_str("Not specified\n");
            }
        },
        serde_json::Value::String(s) => {
            content.push_str(&format!("{}\n", s));
        },
        _ => content.push_str("Not specified\n"),
    }
    
    // Homepage
    if let Some(homepage) = package_info["homepage"].as_str() {
        content.push_str(&format!("- **Homepage:** {}\n", homepage));
    }
    
    // License
    if let Some(license) = package_info["license"].as_str() {
        content.push_str(&format!("- **License:** {}\n", license));
    }
    
    // Keywords
    if let Some(kw_array) = package_info["keywords"].as_array() {
        if !kw_array.is_empty() {
            let keywords: Vec<&str> = kw_array.iter()
                .filter_map(|k| k.as_str())
                .collect();
            content.push_str(&format!("- **Keywords:** {}\n", keywords.join(", ")));
        }
    }
    
    // NPM stats
    content.push_str(&format!("- **NPM URL:** https://www.npmjs.com/package/{}\n", package_name));
    
    // Version history
    if let Some(versions) = package_info["versions"].as_object() {
        content.push_str(&format!("- **Total Versions:** {}\n", versions.len()));
        
        // Add version history section
        content.push_str("\n### Version History\n\n");
        
        let mut version_list: Vec<(&String, &serde_json::Value)> = versions.iter().collect();
        // Sort versions by publish time if available
        version_list.sort_by(|a, b| {
            let a_time = a.1.get("time").and_then(|t| t.as_str()).unwrap_or("");
            let b_time = b.1.get("time").and_then(|t| t.as_str()).unwrap_or("");
            b_time.cmp(a_time) // Descending order (newest first)
        });
        
        // Take only the 10 most recent versions
        content.push_str("| Version | Published | Description |\n");
        content.push_str("|---------|-----------|-------------|\n");
        
        let time_obj = package_info["time"].as_object();
        for (_i, (version, _)) in version_list.iter().enumerate().take(10) {
            let published = if let Some(time) = time_obj.and_then(|t| t.get(*version)).and_then(|d| d.as_str()) {
                time
            } else {
                "Unknown"
            };
            
            let ver_desc = if let Some(ver_obj) = versions.get(*version) {
                ver_obj.get("description").and_then(|d| d.as_str()).unwrap_or("")
            } else {
                ""
            };
            
            content.push_str(&format!("| {} | {} | {} |\n", version, published, ver_desc));
        }
        content.push_str("\n");
    }
    
    // Dependencies section
    content.push_str("## Dependencies\n\n");
    
    // Regular dependencies
    if let Some(deps) = package_info["versions"][latest_version]["dependencies"].as_object() {
        if deps.is_empty() {
            content.push_str("*No dependencies*\n\n");
        } else {
            content.push_str("### Runtime Dependencies\n\n");
            content.push_str("| Package | Version Requirement |\n");
            content.push_str("|---------|---------------------|\n");
            
            let mut dep_list: Vec<(&String, &serde_json::Value)> = deps.iter().collect();
            dep_list.sort_by(|a, b| a.0.cmp(b.0));
            
            for (dep_name, dep_ver) in dep_list {
                let version = match dep_ver {
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::from("unknown")
                };
                content.push_str(&format!("| {} | {} |\n", dep_name, version));
            }
            content.push_str("\n");
        }
    }
    
    // Dev dependencies
    if let Some(dev_deps) = package_info["versions"][latest_version]["devDependencies"].as_object() {
        if !dev_deps.is_empty() {
            content.push_str("### Development Dependencies\n\n");
            content.push_str("| Package | Version Requirement |\n");
            content.push_str("|---------|---------------------|\n");
            
            let mut dev_dep_list: Vec<(&String, &serde_json::Value)> = dev_deps.iter().collect();
            dev_dep_list.sort_by(|a, b| a.0.cmp(b.0));
            
            for (dep_name, dep_ver) in dev_dep_list {
                let version = match dep_ver {
                    serde_json::Value::String(s) => s.clone(),
                    _ => String::from("unknown")
                };
                content.push_str(&format!("| {} | {} |\n", dep_name, version));
            }
            content.push_str("\n");
        }
    }
    
    // Download and extract the package
    pb.set_message(format!("Downloading and extracting package: {}", package_name));
    
    // Get tarball URL
    let tarball_url = if let Some(url) = package_info["versions"][latest_version]["dist"]["tarball"].as_str() {
        url
    } else {
        return Err(ProcessorError::Message("Tarball URL not found in package info".to_string()));
    };
    
    // Download tarball
    let response = client.get(tarball_url).send().await?;
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to download package tarball: HTTP {}", 
            response.status()
        )));
    }
    
    let tarball_bytes = response.bytes().await?;
    let package_path = temp_dir.path().join(format!("{}-{}.tgz", package_name, latest_version));
    fs::write(&package_path, &tarball_bytes).await?;
    
    // Extract tarball
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&extract_dir).await?;
    
    // Use system tar command which handles .tgz files more reliably
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
            "Failed to extract package: exit code {:?}", status.code()
        )));
    }
    
    // Find the package.json in extracted files
    pb.set_message("Processing package content...");
    
    // In most npm packages, the content is in a 'package' directory
    let mut package_dir = extract_dir.clone();
    let package_dir_files = fs::read_dir(&extract_dir).await?;
    let mut entries = package_dir_files;
    let mut found_entry = false;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            package_dir = entry.path();
            found_entry = true;
            break;
        }
    }
    
    if !found_entry {
        return Err(ProcessorError::Processing("Could not find package directory".to_string()));
    }
    
    // Add Package.json content
    let package_json_path = package_dir.join("package.json");
    if package_json_path.exists() {
        content.push_str("## Package.json\n\n```json\n");
        let package_json = fs::read_to_string(&package_json_path).await?;
        content.push_str(&package_json);
        content.push_str("\n```\n\n");
    }
    
    // Process README
    content.push_str("## Documentation\n\n");
    
    // Try to find README (case-insensitive)
    let readme_files = [
        "README.md", "README", "README.txt", "README.markdown", "Readme.md", "readme.md"
    ];
    
    let mut found_readme = false;
    for readme_name in readme_files {
        let readme_path = package_dir.join(readme_name);
        if readme_path.exists() {
            found_readme = true;
            content.push_str("### README\n\n");
            let readme_content = fs::read_to_string(&readme_path).await?;
            content.push_str(&readme_content);
            content.push_str("\n\n");
            break;
        }
    }
    
    if !found_readme {
        content.push_str("*No README found*\n\n");
    }
    
    // Process main source files
    content.push_str("## Source Code\n\n");
    
    // Find main entry point
    let main_file = if let Some(main) = package_info["versions"][latest_version]["main"].as_str() {
        package_dir.join(main)
    } else {
        package_dir.join("index.js")
    };
    
    if main_file.exists() {
        content.push_str(&format!("### Main Entry Point: {}\n\n", main_file.file_name().unwrap_or_default().to_string_lossy()));
        content.push_str("```javascript\n");
        let main_content = fs::read_to_string(&main_file).await?;
        content.push_str(&main_content);
        content.push_str("\n```\n\n");
    }
    
    // Find and list key source files
    let mut source_files = Vec::new();
    find_source_files(&package_dir, &mut source_files).await?;
    
    if !source_files.is_empty() {
        // Count file types
        let mut file_type_counts = std::collections::HashMap::new();
        for file in &source_files {
            if let Some(ext) = file.extension() {
                if let Some(ext_str) = ext.to_str() {
                    *file_type_counts.entry(ext_str.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        content.push_str("### File Types\n\n");
        for (ext, count) in file_type_counts {
            content.push_str(&format!("- **{}**: {} files\n", ext, count));
        }
        content.push_str("\n");
        
        // Select important files
        content.push_str("### Key Source Files\n\n");
        let important_files = source_files.clone();
        
        for file in important_files {
            let rel_path = file.strip_prefix(&package_dir).unwrap_or(&file);
            content.push_str(&format!("#### {}\n\n", rel_path.display()));
            content.push_str("```");
            
            // Add language hint based on extension
            if let Some(ext) = file.extension() {
                if let Some(ext_str) = ext.to_str() {
                    match ext_str {
                        "js" => content.push_str("javascript"),
                        "ts" => content.push_str("typescript"),
                        "jsx" => content.push_str("jsx"),
                        "tsx" => content.push_str("tsx"),
                        "json" => content.push_str("json"),
                        "css" => content.push_str("css"),
                        "scss" => content.push_str("scss"),
                        "html" => content.push_str("html"),
                        "md" => content.push_str("markdown"),
                        _ => {}
                    }
                }
            }
            content.push_str("\n");
            
            // Add file content
            let file_content = fs::read_to_string(&file).await?;
            content.push_str(&file_content);
            content.push_str("\n```\n\n");
        }
    } else {
        content.push_str("*No source files found*\n\n");
    }
    
    // Organize and save the content
    let organized_content = common::organize_content(&content, package_name, "npm");
    
    // Save comprehensive output
    let output_path = common::save_comprehensive_output(
        package_name,
        "npm",
        &organized_content,
        output_dir,
        pb
    ).await?;
    
    pb.finish_with_message(format!("[SUCCESS] Package {} processed successfully. Output saved to: {}", 
                               package_name, output_path.display()));
    
    Ok(())
}

/// Find JavaScript/TypeScript source files in a directory
async fn find_source_files(dir: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = fs::read_dir(dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = fs::metadata(&path).await?;
        
        if metadata.is_dir() {
            // Skip node_modules and other common directories to ignore
            let dir_name = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
                
            if dir_name == "node_modules" || dir_name == "dist" || dir_name == "build" || 
               dir_name.starts_with(".") {
                continue;
            }
            
            // Use Box::pin to handle recursion in async function
            Box::pin(find_source_files(&path, result)).await?;
        } else if metadata.is_file() {
            // Check extensions
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    match ext_str {
                        "js" | "ts" | "jsx" | "tsx" | "json" | "css" | "scss" | "html" | "md" => {
                            result.push(path);
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Select important NPM package files
fn select_important_npm_files(files: &[PathBuf], max_files: usize) -> Vec<PathBuf> {
    if files.len() <= max_files {
        return files.to_vec();
    }
    
    // Priority patterns for important files
    let priority_patterns = [
        "index.js", "index.ts", "index.tsx", "index.jsx",
        "main.js", "main.ts", 
        "app.js", "app.ts",
        "package.json", "tsconfig.json",
        "webpack.config.js", "rollup.config.js",
        "src/", "lib/", "dist/",
        "components/", "utils/", "helpers/"
    ];
    
    // Score each file based on priority patterns and path depth
    let mut scored_files: Vec<(i32, &PathBuf)> = files.iter()
        .map(|file| {
            let path_str = file.to_string_lossy();
            
            // Base score - prefer shorter paths (closer to root)
            let depth_score = -(path_str.matches('/').count() as i32);
            
            // Priority score - prefer files matching priority patterns
            let mut priority_score = 0;
            for (idx, pattern) in priority_patterns.iter().enumerate() {
                // Earlier patterns get higher weight
                let weight = priority_patterns.len() - idx;
                if path_str.contains(pattern) {
                    priority_score += weight as i32 * 5;
                }
            }
                
            // Combine scores
            (depth_score + priority_score, file)
        })
        .collect();
        
    // Sort by score (highest first)
    scored_files.sort_by(|a, b| b.0.cmp(&a.0));
    
    // Take top max_files
    scored_files.iter()
        .take(max_files)
        .map(|(_, file)| (*file).clone())
        .collect()
}

/// Information about an NPM package
#[derive(Debug, Serialize, Deserialize)]
struct PackageInfo {
    /// Name of the NPM package
    name: String,
    /// Version of the NPM package
    version: String,
    /// Optional description of the package
    description: Option<String>,
    /// Optional repository URL where the package source is hosted
    repository: Option<String>,
    /// URL to download the package
    download_url: String,
}

/// Saves output content to a file
///
/// # Arguments
/// * `output_path` - Path where the output should be saved
/// * `output` - Content to save to the file
///
/// # Returns
/// Result indicating success or failure
async fn save_output(output_path: &Path, output: String) -> Result<()> {
    fs::write(output_path, output).await?;
    Ok(())
}

mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_npm_processor() {
        let processor = NpmProcessor::new();
        assert!(processor.accepts("https://www.npmjs.com/package/express"));
        assert!(!processor.accepts("https://pypi.org/project/requests"));
    }
    
    #[tokio::test]
    async fn test_validation() {
        let processor = NpmProcessor::new();
        let result1 = processor.validate("https://www.npmjs.com/package/express").await;
        assert!(result1.is_ok());
        
        let result2 = processor.validate("https://www.npmjs.com/user/express").await;
        assert!(result2.is_err());
        
        let result3 = processor.validate("https://pypi.org/project/requests").await;
        assert!(result3.is_err());
    }
} 