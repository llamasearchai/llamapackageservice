use crate::error::{ProcessorError, Result};
use crate::processors::common::{self, setup_progress_style, save_output_file};
use crate::config::Config;
use crate::processors::PackageProcessor;
use async_trait::async_trait;
use chrono::Utc;
use indicatif::ProgressBar;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use time;
use tracing::{debug, info};
use std::fs;
use walkdir::WalkDir;
use tokio;
use scraper::{Html, Selector};
use std::time::Duration;
use serde_json;
use std::sync::{Arc, Mutex};

/// Represents a Go module with its metadata
///
/// Contains information about a Go module including its name,
/// version, Go version requirements, and dependencies.
#[derive(Debug, Serialize, Deserialize)]
pub struct GoModule {
    /// Name of the Go module
    name: String,
    /// Version of the Go module
    version: String,
    /// Required Go language version
    go_version: String,
    /// List of module dependencies
    dependencies: Vec<String>,
}

/// Processor for handling Go packages
///
/// The GoProcessor is responsible for processing Go package URLs,
/// extracting module information, and generating documentation.
#[derive(Debug)]
pub struct GoProcessor {
    /// HTTP client for making requests
    client: Client,
}

impl GoProcessor {
    /// Creates a new Go processor with default configuration
    ///
    /// # Returns
    /// A new GoProcessor instance wrapped in a Result
    pub fn new() -> Result<Self> {
        let client = common::create_client_with_user_agent();
        
        Ok(Self {
            client,
        })
    }

    async fn fetch_module_info(&self, module_path: &str) -> Result<GoModule> {
        let url = format!("https://proxy.golang.org/{module_path}/@v/list");
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| ProcessorError::Http(e))?;

        if !response.status().is_success() {
            return Err(ProcessorError::Message(
                format!("Failed to fetch Go module info: {}", response.status())
            ));
        }

        let versions = response.text()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        // Parse the response and create a GoModule
        let latest_version = versions.lines()
            .last()
            .ok_or_else(|| ProcessorError::Message("No versions found".into()))?;

        Ok(GoModule {
            name: module_path.to_string(),
            version: latest_version.to_string(),
            go_version: "1.16".to_string(),
            dependencies: Vec::new(),
        })
    }
}

impl Clone for GoProcessor {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

#[async_trait]
impl PackageProcessor for GoProcessor {
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        // Create a progress bar (would be provided by the caller in a real app)
        let pb = common::create_progress_bar();
        
        // Process the package using the standalone function
        process_go_package(url, output_dir, &pb).await
    }

    fn name(&self) -> &'static str {
        "go"
    }

    fn accepts(&self, url: &str) -> bool {
        url.contains("pkg.go.dev")
    }

    async fn validate(&self, url: &str) -> Result<()> {
        if !self.accepts(url) {
            return Err(ProcessorError::Message(
                format!("Not a valid pkg.go.dev URL: {}", url)
            ));
        }
        Ok(())
    }
}

/// Extract the package path from a pkg.go.dev URL
fn extract_package_path(url: &str) -> Result<String> {
    let url = url.trim();
    
    if !url.starts_with("https://pkg.go.dev/") && !url.starts_with("http://pkg.go.dev/") {
        return Err(ProcessorError::Message(format!("Not a valid pkg.go.dev URL: {}", url)));
    }
    
    // Remove the prefix
    let path = url.replace("https://pkg.go.dev/", "").replace("http://pkg.go.dev/", "");
    
    // Remove any query parameters or fragments
    let path = path.split('#').next().unwrap_or(&path);
    let path = path.split('?').next().unwrap_or(&path);
    
    if path.is_empty() {
        return Err(ProcessorError::Message("Empty package path".to_string()));
    }
    
    Ok(path.to_string())
}

/// Processes a Go package from a URL
///
/// # Arguments
/// * `package_url` - The Go package URL to process
/// * `output_dir` - Directory to save output to
/// * `pb` - Progress bar for displaying progress
///
/// # Returns
/// Result indicating success or failure
pub async fn process_go_package(
    package_url: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    pb.set_message(format!("Processing Go package: {}", package_url));
    
    // Extract package path from URL
    let package_path = extract_package_path(package_url)?;
    let package_name = package_path.split('/').last().unwrap_or(&package_path);
    
    // Create HTTP client
    let client = Arc::new(reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?);
    
    // Create temp directory for downloaded files
    let _temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;
    
    // Initialize content with package information
    let content = Arc::new(Mutex::new(String::new()));
    
    // Package information section
    {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str("## Package Information\n\n");
        content_guard.push_str(&format!("- **Name**: {}\n", package_name));
        content_guard.push_str(&format!("- **Import Path**: {}\n", package_path));
    }
    
    // Fetch Go package documentation from pkg.go.dev
    pb.set_message(format!("Fetching documentation for: {}", package_path));
    
    let doc_url = format!("https://pkg.go.dev/{}", package_path);
    let response = client.get(&doc_url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to fetch package documentation: HTTP {}", 
            response.status()
        )));
    }
    
    let html = response.text().await?;
    
    // Extract all the information we need from the HTML upfront
    let description;
    let version;
    let license;
    let api_docs;
    let examples;
    let dependencies;
    let repository_url;
    
    // Use a scope to ensure the HTML document doesn't escape
    {
        // Parse the HTML document in a limited scope so it doesn't escape
        let doc = Html::parse_document(&html);
        
        // Extract all information we'll need - clone any needed strings to ensure they outlive the document
        description = extract_description(&doc).map(|s| s.to_string());
        version = extract_version(&doc).map(|s| s.to_string());
        license = extract_license(&doc).map(|s| s.to_string());
        api_docs = extract_api_documentation(&doc);
        examples = extract_examples(&doc); 
        dependencies = extract_dependencies(&doc);
        repository_url = extract_repository(&doc).map(|s| s.to_string());
    } // HTML document is dropped here
    
    // Now add the content with the extracted information
    
    // Add description
    if let Some(description) = &description {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str(&format!("- **Description**: {}\n", description.trim()));
    }
    
    // Add version information
    if let Some(version) = &version {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str(&format!("- **Version**: {}\n", version.trim()));
    }
    
    // Extract license information
    if let Some(license) = &license {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str(&format!("- **License**: {}\n", license.trim()));
    }
    
    // Add API documentation if available
    if !api_docs.is_empty() {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str("\n## API Documentation\n\n");
        content_guard.push_str(&api_docs);
    }
    
    // Add examples if available
    if !examples.is_empty() {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str("\n## Examples\n\n");
        content_guard.push_str(&examples);
    }
    
    // Add dependencies if available
    if !dependencies.is_empty() {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str("\n## Dependencies\n\n");
        
        for dependency in &dependencies {
            content_guard.push_str(&format!("- {}\n", dependency));
        }
    } else {
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str("No dependency information found.\n");
    }
    
    // Try to get repository information
    if let Some(repo_url) = repository_url {
        // Use the helper function to fetch repository data
        // This ensures we don't hold HTML references across await points
        fetch_repository_data(&repo_url, &client, &content, &pb).await?;
    }
    
    // Organize and enhance the content
    let organized_content = common::organize_content(&content.lock().unwrap(), package_name, "go");
    
    // Save comprehensive output
    let _output_path = common::save_comprehensive_output(
        package_name,
        "go",
        &organized_content,
        output_dir,
        pb
    ).await?;
    
    pb.finish_with_message(format!("✨ Go package {} processed successfully", package_name));
    Ok(())
}

// Helper functions for extracting information from HTML

fn extract_description(document: &Html) -> Option<String> {
    let selector = Selector::parse(".Documentation-overview p").ok()?;
    document.select(&selector).next().map(|element| element.text().collect::<String>())
}

fn extract_version(document: &Html) -> Option<String> {
    let selector = Selector::parse(".DetailsHeader-version").ok()?;
    document.select(&selector).next().map(|element| element.text().collect::<String>())
}

fn extract_license(document: &Html) -> Option<String> {
    let selector = Selector::parse(".DetailsHeader-license").ok()?;
    document.select(&selector).next().map(|element| element.text().collect::<String>())
}

fn extract_api_documentation(document: &Html) -> String {
    let mut result = String::new();
    
    // Extract types
    if let Ok(type_selector) = Selector::parse(".Documentation-types .js-typeDetails") {
        let mut has_types = false;
        for element in document.select(&type_selector) {
            has_types = true;
            
            // Extract type name
            if let Ok(name_selector) = Selector::parse(".Documentation-typeHeader") {
                if let Some(name_element) = element.select(&name_selector).next() {
                    result.push_str(&format!("### Type: {}\n\n", name_element.text().collect::<String>().trim()));
                }
            }
            
            // Extract type description
            if let Ok(desc_selector) = Selector::parse(".Documentation-typeDescription") {
                if let Some(desc_element) = element.select(&desc_selector).next() {
                    result.push_str(&format!("{}\n\n", desc_element.text().collect::<String>().trim()));
                }
            }
            
            // Extract methods
            if let Ok(method_selector) = Selector::parse(".Documentation-typeMethods .Documentation-function") {
                for method in element.select(&method_selector) {
                    if let Ok(method_name_selector) = Selector::parse(".Documentation-functionHeader") {
                        if let Some(method_name) = method.select(&method_name_selector).next() {
                            result.push_str(&format!("#### Method: `{}`\n\n", method_name.text().collect::<String>().trim()));
                        }
                    }
                    
                    if let Ok(method_desc_selector) = Selector::parse(".Documentation-functionDoc") {
                        if let Some(method_desc) = method.select(&method_desc_selector).next() {
                            result.push_str(&format!("{}\n\n", method_desc.text().collect::<String>().trim()));
                        }
                    }
                }
            }
        }
        
        if has_types {
            result.push_str("\n");
        }
    }
    
    // Extract functions
    if let Ok(func_selector) = Selector::parse(".Documentation-function") {
        let mut has_functions = false;
        
        for element in document.select(&func_selector) {
            has_functions = true;
            
            // Extract function signature
            if let Ok(signature_selector) = Selector::parse(".Documentation-functionHeader") {
                if let Some(signature) = element.select(&signature_selector).next() {
                    result.push_str(&format!("### Function: `{}`\n\n", signature.text().collect::<String>().trim()));
                }
            }
            
            // Extract function description
            if let Ok(desc_selector) = Selector::parse(".Documentation-functionDoc") {
                if let Some(desc) = element.select(&desc_selector).next() {
                    result.push_str(&format!("{}\n\n", desc.text().collect::<String>().trim()));
                }
            }
        }
        
        if !has_functions {
            result.push_str("No functions found in documentation.\n");
        }
    }
    
    result
}

fn extract_examples(document: &Html) -> String {
    let mut result = String::new();
    
    if let Ok(example_selector) = Selector::parse("#example") {
        if let Some(example_section) = document.select(&example_selector).next() {
            if let Ok(code_selector) = Selector::parse("pre") {
                for (i, code_element) in example_section.select(&code_selector).enumerate() {
                    result.push_str(&format!("### Example {}\n\n", i + 1));
                    result.push_str("```go\n");
                    result.push_str(&code_element.text().collect::<String>());
                    result.push_str("\n```\n\n");
                }
            }
        }
    }
    
    result
}

fn extract_dependencies(document: &Html) -> Vec<String> {
    let mut dependencies = Vec::new();
    
    if let Ok(imports_selector) = Selector::parse(".Imports-list a") {
        for element in document.select(&imports_selector) {
            if let Some(href) = element.value().attr("href") {
                if href.starts_with("/") {
                    // Convert relative URL to package path
                    let path = href.trim_start_matches("/");
                    dependencies.push(path.to_string());
                }
            }
        }
    }
    
    dependencies
}

fn extract_repository(document: &Html) -> Option<String> {
    if let Ok(repo_selector) = Selector::parse("a[data-test-id=\"UnitDirectoryHeader-repoURL\"]") {
        if let Some(element) = document.select(&repo_selector).next() {
            return element.value().attr("href").map(|s| s.to_string());
        }
    }
    
    None
}

// Helper functions below

fn walk_directory(dir: &Path) -> Result<Vec<PathBuf>> {
    let files = WalkDir::new(dir)
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(|e| e.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .collect::<Vec<_>>();
        
    Ok(files)
}

fn generate_go_package_index(dir_path: &Path, package_name: &str) -> Result<String> {
    let mut content = String::new();
    content.push_str(&format!("# Go Package: {}\n\n", package_name));
    
    // Add basic information
    content.push_str("## Package Information\n\n");
    
    // Add file listing
    content.push_str("## Files\n\n");
    let files = WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(|e| e.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .collect::<Vec<_>>();
        
    for file in &files {
        if let Some(path) = file.strip_prefix(dir_path).ok() {
            content.push_str(&format!("- {}\n", path.display()));
        }
    }
    
    // Look for go.mod file
    let go_mod_path = dir_path.join("go.mod");
    if go_mod_path.exists() {
        content.push_str("\n## Dependencies (from go.mod)\n\n");
        if let Ok(go_mod_content) = fs::read_to_string(go_mod_path) {
            content.push_str("```\n");
            content.push_str(&go_mod_content);
            content.push_str("\n```\n");
        }
    }
    
    Ok(content)
}

/// A helper function to fetch repository data without holding HTML references
async fn fetch_repository_data(repo_url: &str, client: &Client, content: &Arc<Mutex<String>>, pb: &ProgressBar) -> Result<()> {
    // Attempt to fetch repository content if it's GitHub
    if repo_url.contains("github.com") {
        pb.set_message("Fetching repository information...");
        
        // Extract owner and repo name from URL
        let parts: Vec<&str> = repo_url.split('/').collect();
        if parts.len() >= 5 {
            let owner = parts[3];
            let repo = parts[4].trim_end_matches(".git");
            
            // Fetch repository information
            let repo_api_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
            match client.get(&repo_api_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(repo_info) = response.json::<serde_json::Value>().await {
                            // Extract all the data we need from the JSON before locking the mutex
                            let mut lines = Vec::new();
                            lines.push("\n### Repository Information\n\n".to_string());
                            
                            if let Some(stars) = repo_info["stargazers_count"].as_u64() {
                                lines.push(format!("- **Stars**: {}\n", stars));
                            }
                            
                            if let Some(forks) = repo_info["forks_count"].as_u64() {
                                lines.push(format!("- **Forks**: {}\n", forks));
                            }
                            
                            if let Some(desc) = repo_info["description"].as_str() {
                                lines.push(format!("- **Description**: {}\n", desc));
                            }
                            
                            if let Some(homepage) = repo_info["homepage"].as_str() {
                                if !homepage.is_empty() {
                                    lines.push(format!("- **Homepage**: {}\n", homepage));
                                }
                            }
                            
                            // Now lock the mutex but only for a short time to update the content
                            let mut content_guard = content.lock().unwrap();
                            for line in lines {
                                content_guard.push_str(&line);
                            }
                        }
                    }
                },
                Err(e) => {
                    let mut content_guard = content.lock().unwrap();
                    content_guard.push_str(&format!("\nFailed to fetch repository information: {}\n", e));
                }
            }
        }
    } else {
        // Just add the repository URL
        let mut content_guard = content.lock().unwrap();
        content_guard.push_str(&format!("\n## Repository\n\n{}\n", repo_url));
    }
    
    Ok(())
} 