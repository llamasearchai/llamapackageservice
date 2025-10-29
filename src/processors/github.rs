use crate::error::{ProcessorError, Result};
use crate::config::Config;
use crate::processors::common::{
    self, check_rate_limit, download_file, 
    extract_archive as common_extract_archive, 
    save_output_file, 
    setup_progress_style as common_setup_progress_style,
    create_progress_bar
};
use crate::processors::PackageProcessor;
use std::fs as std_fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use std::io::{self, Cursor};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::TryFutureExt;
use futures_util::StreamExt as FuturesStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use reqwest::{Client, StatusCode};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tempfile::tempdir;
use tokio::fs as tokio_fs;
use tokio::time::sleep;
use url::Url;
use walkdir::WalkDir;
use zip::ZipArchive;
use std::fs::File;
use std::io::Read;
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt as TokioStreamExt;
use std::sync::Arc;
use std::error::Error as StdError;
use std::path;

const RETRY_DELAY_MS: u64 = 1000;
const MAX_RETRIES: u32 = 3;
const NOTEBOOK_EXTENSIONS: &[&str] = &[".ipynb"];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepoInfo {
    name: String,
    owner: String,
    description: Option<String>,
    language: Option<String>,
    stars: u32,
    forks: u32,
    default_branch: String,
}

// Constants for file types and processing
const COMMON_BRANCHES: &[&str] = &["main", "master", "dev", "development", "stable"];
const CODE_EXTENSIONS: &[&str; 36] = &[
    // Systems programming languages
    ".rs", ".go", ".c", ".cpp", ".h", ".hpp",
    // Web development
    ".js", ".ts", ".jsx", ".tsx", ".html", ".css", ".scss", ".sass",
    // Scripting languages
    ".py", ".rb", ".php", ".sh", ".bash", ".zsh",
    // JVM languages
    ".java", ".kt", ".scala", ".groovy", ".clj",
    // .NET languages
    ".cs", ".fs", ".vb",
    // Mobile development
    ".swift", ".m", ".mm",
    // Configuration and data
    ".json", ".yml", ".yaml", ".toml", ".md"
];

// File organization constants
const GITHUB_INDIVIDUAL_REPOS_DIR: &str = "github_individual_repos";
const GITHUB_ORGANIZATIONS_DIR: &str = "github_organizations";
const ANALYSIS_SUFFIX: &str = "analysis";
const OVERVIEW_SUFFIX: &str = "overview";

// Rate limiting and timeout constants
const API_TIMEOUT_SECS: u64 = 30;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_BASE: &str = "https://github.com";
const GITHUB_ORG_BASE: &str = "https://github.com/orgs";

/// Information about a GitHub repository
///
/// Contains key metadata about a GitHub repository including owner, name,
/// description, primary language, and engagement metrics.
#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Owner/organization name for the repository
    owner: String,
    /// Name of the repository
    repo: String,
    /// Optional description of the repository
    description: Option<String>,
    /// Primary programming language used in the repository
    language: Option<String>,
    /// Number of GitHub stars (popularity metric)
    stars: u64,
    /// Number of repository forks
    forks: u64,
}

/// Information about a GitHub organization
///
/// Contains details about a GitHub organization including its name,
/// description, and aggregate statistics about its repositories.
#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationInfo {
    /// Name of the GitHub organization
    name: String,
    /// Optional description of the organization
    description: Option<String>,
    /// Number of repositories in the organization
    repo_count: usize,
    /// Sum of stars across all organization repositories
    total_stars: u64,
}

// Only define GitHubUrlType once
#[derive(Debug)]
enum GitHubUrlType {
    Repository { owner: String, repo: String },
    Organization { org: String },
}

#[derive(Debug)]
struct OutputStructure {
    base_dir: PathBuf,
    individual_repos_dir: PathBuf,
    organizations_dir: PathBuf,
}

impl OutputStructure {
    async fn new(base_dir: &Path) -> Result<Self> {
        let individual_repos_dir = base_dir.join(GITHUB_INDIVIDUAL_REPOS_DIR);
        let organizations_dir = base_dir.join(GITHUB_ORGANIZATIONS_DIR);

        // Create directories if they don't exist
        tokio_fs::create_dir_all(&individual_repos_dir).await?;
        tokio_fs::create_dir_all(&organizations_dir).await?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            individual_repos_dir,
            organizations_dir,
        })
    }

    fn get_repo_output_path(&self, owner: &str, repo: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_{}_{}_{}.txt", 
            timestamp,
            sanitize_filename(owner),
            sanitize_filename(repo),
            get_repo_language(repo),
            ANALYSIS_SUFFIX
        );
        self.individual_repos_dir.join(filename)
    }

    fn get_org_dir(&self, org: &str) -> PathBuf {
        self.organizations_dir.join(sanitize_filename(org))
    }

    fn get_org_overview_path(&self, org: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_{}_{}.txt",
            timestamp,
            sanitize_filename(org),
            "organization",
            OVERVIEW_SUFFIX
        );
        self.get_org_dir(org).join(filename)
    }

    fn get_repo_overview_path(&self, owner: &str, repo: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_{}_{}.txt",
            timestamp,
            sanitize_filename(owner),
            sanitize_filename(repo),
            OVERVIEW_SUFFIX
        );
        self.individual_repos_dir.join(filename)
    }

    fn get_org_repo_path(&self, org: &str, repo: &str, language: Option<&str>) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let lang_suffix = language.unwrap_or("unknown");
        let filename = format!(
            "{}_{}_{}_{}_{}.txt",
            timestamp,
            sanitize_filename(org),
            sanitize_filename(repo),
            lang_suffix,
            ANALYSIS_SUFFIX
        );
        self.get_org_dir(org).join(filename)
    }
}

// Helper functions for file organization
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn get_repo_language(repo: &str) -> String {
    // Detect primary language from repository name or return "unknown"
    for ext in CODE_EXTENSIONS.iter() {
        if repo.to_lowercase().ends_with(ext) {
            return ext[1..].to_string(); // Remove the dot
        }
    }
    "unknown".to_string()
}

/// Processes a GitHub URL and determines if it's a repository or organization.
/// Calls the appropriate processing function based on the URL type.
pub async fn process_github_url(
    url: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    common::setup_progress_style(pb);
    pb.set_message("Processing GitHub URL...");

    let url_type = determine_github_url_type(url).await?;
    
    match url_type {
        GitHubUrlType::Repository { owner, repo } => {
            process_single_repo(&owner, &repo, url, output_dir, pb).await?;
        }
        GitHubUrlType::Organization { org } => {
            process_org(&org, output_dir, pb).await?;
        }
    }

    pb.finish_with_message(format!("✨ GitHub URL {} processed successfully", url));
    Ok(())
}

async fn determine_github_url_type(url: &str) -> Result<GitHubUrlType> {
    let parsed_url = Url::parse(url)
        .map_err(|_| ProcessorError::Validation("Invalid URL format".into()))?;
    
    if parsed_url.host_str() != Some("github.com") {
        return Err(ProcessorError::Validation("Not a GitHub URL".into()));
    }

    let segments: Vec<&str> = parsed_url.path_segments()
        .ok_or_else(|| ProcessorError::Validation("Invalid URL path".into()))?
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        return Err(ProcessorError::Validation("Invalid GitHub URL".into()));
    }

    // Create a client to check the URL type
    let client = create_github_client()?;

    // Try to fetch as repository first
    if segments.len() >= 2 {
        let owner = segments[0];
        let repo = segments[1];
        
        // Check if it's a valid repository
        let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = client.get(&repo_url)
            .header("User-Agent", "LlamaSearch")
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        if response.status() == StatusCode::OK {
            return Ok(GitHubUrlType::Repository {
                owner: owner.to_string(),
                repo: repo.to_string(),
            });
        }
    }

    // If not a repository, check if it's an organization
    let org = segments[0];
    let org_url = format!("https://api.github.com/orgs/{}", org);
    let response = client.get(&org_url)
        .header("User-Agent", "LlamaSearch")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;

    if response.status() == StatusCode::OK {
        Ok(GitHubUrlType::Organization {
            org: org.to_string(),
        })
    } else {
        Err(ProcessorError::Validation("Invalid GitHub URL or resource not found".into()))
    }
}

/// Processes a GitHub organization
async fn process_org(
    org: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    let output_structure = OutputStructure::new(output_dir).await?;
    let org_dir = output_structure.get_org_dir(org);
    tokio_fs::create_dir_all(&org_dir)
        .await
        .map_err(|e| ProcessorError::IO(e))?;

    pb.set_message(format!("Processing GitHub organization: {}", org));
    
    let client = create_github_client()?;

    // Initialize content for the comprehensive output
    let mut content = String::new();
    
    // Process organization overview
    content.push_str(&format!("## GitHub Organization: {}\n\n", org));
    content.push_str(&format!("- **URL:** https://github.com/{}\n", org));
    content.push_str(&format!("- **Processed:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    // Get organization info
    let org_info = fetch_org_info(&client, org).await?;
    
    // Add organization details to overview
    if let Some(description) = org_info["description"].as_str() {
        content.push_str(&format!("- **Description:** {}\n", description));
    }
    if let Some(location) = org_info["location"].as_str() {
        content.push_str(&format!("- **Location:** {}\n", location));
    }
    if let Some(blog) = org_info["blog"].as_str() {
        content.push_str(&format!("- **Website:** {}\n", blog));
    }
    if let Some(twitter) = org_info["twitter_username"].as_str() {
        content.push_str(&format!("- **Twitter:** @{}\n", twitter));
    }
    if let Some(email) = org_info["email"].as_str() {
        content.push_str(&format!("- **Email:** {}\n", email));
    }
    content.push_str("\n");

    // Get repositories
    let repos = fetch_org_repos(&client, org).await?;
    
    content.push_str(&format!("- **Total Public Repositories:** {}\n\n", repos.len()));
    
    // Add language statistics
    let mut language_stats = std::collections::HashMap::new();
    let mut total_stars = 0;
    let mut total_forks = 0;
    
    for repo in &repos {
        if let Some(language) = repo["language"].as_str() {
            *language_stats.entry(language.to_string()).or_insert(0) += 1;
        }
        total_stars += repo["stargazers_count"].as_u64().unwrap_or(0);
        total_forks += repo["forks_count"].as_u64().unwrap_or(0);
    }
    
    content.push_str(&format!("- **Total Stars:** {}\n", total_stars));
    content.push_str(&format!("- **Total Forks:** {}\n\n", total_forks));
    
    content.push_str("## Language Distribution\n\n");
    let mut langs: Vec<_> = language_stats.iter().collect();
    langs.sort_by(|a, b| b.1.cmp(a.1));
    
    for (language, count) in langs {
        let percentage = (*count as f64 / repos.len() as f64) * 100.0;
        content.push_str(&format!("- **{}**: {} repositories ({:.1}%)\n", language, count, percentage));
    }
    content.push_str("\n");
    
    // List top repositories by stars
    content.push_str("## Top Repositories\n\n");
    let mut sorted_repos = repos.clone();
    sorted_repos.sort_by(|a, b| {
        let a_stars = a["stargazers_count"].as_u64().unwrap_or(0);
        let b_stars = b["stargazers_count"].as_u64().unwrap_or(0);
        b_stars.cmp(&a_stars)
    });
    
    content.push_str("| Repository | Description | Stars | Forks | Language |\n");
    content.push_str("|------------|-------------|-------|-------|----------|\n");
    
    for repo in sorted_repos.iter().take(20) {
        let name = repo["name"].as_str().unwrap_or("Unknown");
        let description = repo["description"].as_str().unwrap_or("").replace("|", "\\|");
        let truncated_desc = if description.len() > 80 {
            format!("{}...", &description[..77])
        } else {
            description
        };
        let stars = repo["stargazers_count"].as_u64().unwrap_or(0);
        let forks = repo["forks_count"].as_u64().unwrap_or(0);
        let language = repo["language"].as_str().unwrap_or("Unknown");
        
        content.push_str(&format!("| [{}]({}/{}) | {} | {} | {} | {} |\n", 
            name, GITHUB_BASE, org, truncated_desc, stars, forks, language));
    }
    content.push_str("\n");
    
    // Process top repositories in detail
    content.push_str("## Detailed Repository Information\n\n");
    
    let top_repos = sorted_repos.iter().take(5);
    let mut repo_count = 0;
    
    for repo in top_repos {
        let repo_name = repo["name"].as_str().unwrap_or("unknown");
        pb.set_message(format!("Processing repository {}/{} ({}/{})", org, repo_name, repo_count + 1, 5));
        
        content.push_str(&format!("### {}\n\n", repo_name));
        
        if let Some(description) = repo["description"].as_str() {
            content.push_str(&format!("**Description:** {}\n\n", description));
        }
        
        content.push_str(&format!("**URL:** https://github.com/{}/{}\n", org, repo_name));
        content.push_str(&format!("**Stars:** {}\n", repo["stargazers_count"].as_u64().unwrap_or(0)));
        content.push_str(&format!("**Forks:** {}\n", repo["forks_count"].as_u64().unwrap_or(0)));
        
        if let Some(language) = repo["language"].as_str() {
            content.push_str(&format!("**Primary Language:** {}\n", language));
        }
        
        if let Some(license) = repo["license"].as_object() {
            if let Some(name) = license.get("name").and_then(|n| n.as_str()) {
                content.push_str(&format!("**License:** {}\n", name));
            }
        }
        
        // Try to get README content for this repository
        let _default_branch = repo["default_branch"].as_str().unwrap_or("main");
        
        let readme_url = format!(
            "{}/repos/{}/{}/readme",
            GITHUB_API_BASE,
            org,
            repo_name
        );
        
        let readme_response = client.get(&readme_url)
            .header("Accept", "application/vnd.github.v3.raw")
            .send()
            .await;
            
        if let Ok(response) = readme_response {
            if response.status().is_success() {
                if let Ok(readme_content) = response.text().await {
                    content.push_str("\n#### README Excerpt\n\n");
                    
                    // Truncate very long READMEs
                    let max_chars = 2000;
                    let truncated = if readme_content.len() > max_chars {
                        format!("{}... (truncated)", &readme_content[..max_chars])
                    } else {
                        readme_content
                    };
                    
                    content.push_str(&truncated);
                    content.push_str("\n\n");
                }
            }
        }
        
        // Add separation between repositories
        content.push_str("\n---\n\n");
        repo_count += 1;
    }
    
    // Organize and save the content
    let organized_content = common::organize_content(&content, org, "github_org");
    
    // Create timestamp for unique filename
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let output_filename = format!("{}_github_org_{}.txt", org, timestamp);
    let output_path = output_dir.join(&output_filename);
    
    // Save output
    tokio_fs::write(&output_path, &organized_content)
        .await
        .map_err(|e| ProcessorError::IO(e))?;
        
    // Also save as .md for better viewing
    let md_path = output_dir.join(format!("{}.md", org));
    tokio_fs::write(&md_path, &organized_content)
        .await
        .map_err(|e| ProcessorError::IO(e))?;
    
    pb.finish_with_message(format!("[SUCCESS] Organization {} processed successfully. Output saved to: {}", 
                                 org, output_path.display()));
    Ok(())
}

async fn process_single_repo_content(
    client: &Client,
    owner: &str,
    repo: &str,
    repo_info: &Value,
) -> Result<String> {
    let mut content = String::new();
    content.push_str(&format!("# Repository: {}/{}\n\n", owner, repo));

    // Add repository information
    if let Some(description) = repo_info["description"].as_str() {
        content.push_str(&format!("Description: {}\n\n", description));
    }
    if let Some(stars) = repo_info["stargazers_count"].as_u64() {
        content.push_str(&format!("Stars: {}\n", stars));
    }
    if let Some(forks) = repo_info["forks_count"].as_u64() {
        content.push_str(&format!("Forks: {}\n", forks));
    }
    if let Some(language) = repo_info["language"].as_str() {
        content.push_str(&format!("Primary Language: {}\n", language));
    }
    
    content.push_str("\n---\n\n");
    
    // Add additional repository analysis
    let repo_details = fetch_repo_info(client, owner, repo).await?;
    if let Some(topics) = repo_details["topics"].as_array() {
        content.push_str("\n## Topics\n");
        for topic in topics {
            if let Some(topic_str) = topic.as_str() {
                content.push_str(&format!("- {}\n", topic_str));
            }
        }
    }

    Ok(content)
}

/// Processes an individual GitHub repository URL.
/// This function downloads, extracts, generates an index, and saves a file for individual repos.
async fn process_single_repo(
    owner: &str,
    repo: &str,
    _repo_url: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    pb.set_message(format!("Processing repository: {}/{}", owner, repo));
    
    // Create client without using create_github_client_with_retry
    let client = create_github_client()?;
    
    // Ensure output structure exists
    let output = OutputStructure::new(output_dir).await?;
    
    // Get repository details
    let repo_details = match client
        .get(&format!("https://api.github.com/repos/{}/{}", owner, repo))
        .header("User-Agent", "llama-package-service")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                response.json::<Value>().await.map_err(|e| ProcessorError::Processing(e.to_string()))?
            } else {
                return Err(ProcessorError::Message(format!(
                    "Failed to get repository details: {}",
                    response.status()
                )));
            }
        }
        Err(e) => return Err(ProcessorError::Http(e)),
    };
    
    // Extract repository information
    let default_branch = repo_details["default_branch"].as_str().unwrap_or("main");
    let url = format!("https://github.com/{}/{}", owner, repo);
    
    // Add language info to the output path
    let lang = repo_details["language"].as_str().unwrap_or("unknown");
    
    // Download repo content
    let repo_content = process_single_repo_content(&client, owner, repo, &repo_details).await?;
    let repo_path = output.get_repo_output_path(owner, repo);
    
    // Create any necessary directories
    if let Some(parent) = repo_path.parent() {
        tokio_fs::create_dir_all(parent).await.map_err(|e| ProcessorError::IO(e))?;
    }
    
    // Write the content
    tokio_fs::write(&repo_path, repo_content).map_err(|e| ProcessorError::IO(e)).await?;
    
    // Create overview file
    let description = repo_details["description"].as_str().unwrap_or("No description");
    let stars = repo_details["stargazers_count"].as_u64().unwrap_or(0);
    let forks = repo_details["forks_count"].as_u64().unwrap_or(0);
    
    let mut overview_content = format!(
        "# Repository: {}/{}\n\n",
        owner, repo
    );
    overview_content.push_str(&format!("URL: {}\n", url));
    overview_content.push_str(&format!("Description: {}\n", description));
    overview_content.push_str(&format!("Primary Language: {}\n", lang));
    overview_content.push_str(&format!("Stars: {}\n", stars));
    overview_content.push_str(&format!("Forks: {}\n", forks));
    overview_content.push_str(&format!("Default Branch: {}\n\n", default_branch));
    
    // Add topics if available
    overview_content.push_str("## Topics\n\n");
    if let Some(topics) = repo_details.get("topics").and_then(|t| t.as_array()) {
        for topic in topics {
            if let Some(topic_str) = topic.as_str() {
                overview_content.push_str(&format!("- {}\n", topic_str));
            }
        }
    } else {
        overview_content.push_str("No topics found\n");
    }
    
    // Add license info if available
    overview_content.push_str("\n## License\n\n");
    if let Some(license) = repo_details.get("license").and_then(|l| l.as_object()) {
        if let Some(license_name) = license.get("name").and_then(|n| n.as_str()) {
            overview_content.push_str(&format!("{}\n", license_name));
        }
    } else {
        overview_content.push_str("No license information available\n");
    }
    
    // Save overview
    let overview_path = output.get_repo_overview_path(owner, repo);
    tokio_fs::write(&overview_path, overview_content).map_err(|e| ProcessorError::IO(e)).await?;
    
    pb.finish_with_message(format!("✨ Repository {}/{} processed successfully", owner, repo));
    Ok(())
}

async fn fetch_repo_info(client: &Client, owner: &str, repo: &str) -> Result<Value> {
    let url = format!("{}/repos/{}/{}", github_api_base(), owner, repo);
    
    let response = client
        .get(&url)
        .header("User-Agent", "llama-package-service")
        .send()
        .await
        .map_err(|e| ProcessorError::Http(e))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Message(format!(
            "Failed to fetch repo info: HTTP {}",
            response.status()
        )));
    }
    
    response
        .json::<Value>()
        .await
        .map_err(|e| ProcessorError::Processing(e.to_string()))
}

async fn fetch_org_info(client: &Client, org: &str) -> Result<Value> {
    let url = format!("{}/orgs/{}", github_api_base(), org);
    make_github_request(client, &url).await
}

async fn fetch_org_repos(client: &Client, org: &str) -> Result<Vec<Value>> {
    let mut all_repos = Vec::new();
    let mut page = 1;
    
    loop {
        let url = format!(
            "{}/orgs/{}/repos?page={}&per_page=100&type=public",
            github_api_base(),
            org,
            page
        );
        
        let response = make_github_request(client, &url).await?;
        
        match response.as_array() {
            Some(repos) => {
                if repos.is_empty() {
                    break;
                }
                all_repos.extend(repos.clone());
                page += 1;
            }
            None => return Err(ProcessorError::Processing("Invalid response format".into()))
        }
    }
    
    Ok(all_repos)
}

async fn download_repo_archive(
    client: &Client,
    owner: &str,
    repo: &str,
    branch: &str
) -> Result<Vec<u8>> {
    let archive_url = format!(
        "https://github.com/{}/{}/archive/refs/heads/{}.zip",
        owner, repo, branch
    );
    
    let response = client.get(&archive_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ProcessorError::Network(format!(
            "Failed to download archive: HTTP {}",
            response.status()
        )));
    }

    response.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| ProcessorError::Network(e.to_string()))
}

// Helper functions for processing repository contents
async fn extract_archive(archive_bytes: &[u8], extract_path: &Path) -> Result<()> {
    // First create the directory
    tokio_fs::create_dir_all(extract_path).await?;
    
    // Use a blocking task to handle zip extraction which isn't async-friendly
    let bytes = archive_bytes.to_vec(); // Clone the bytes
    let path = extract_path.to_path_buf(); // Clone the path
    
    // Use tokio::task::spawn_blocking for CPU-bound operations
    tokio::task::spawn_blocking(move || -> Result<()> {
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path::Path::new(&path).to_path_buf(),
                None => continue,
            };
            
            let outpath = path.join(outpath);
            
            if file.name().ends_with('/') {
                // Directory
                std_fs::create_dir_all(&outpath)?;
            } else {
                // File
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std_fs::create_dir_all(p)?;
                    }
                }
                
                let mut outfile = std_fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        
        Ok(())
    }).await.map_err(|e| ProcessorError::Processing(format!("Join error: {}", e)))?
}

async fn process_readme(repo_path: &Path) -> Result<String> {
    let mut content = String::new();
    let mut entries = tokio_fs::read_dir(repo_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            if let Some(fname) = entry.file_name().to_str() {
                if fname.to_lowercase() == "readme.md" {
                    content = tokio_fs::read_to_string(entry.path()).await?;
                    break;
                }
            }
        }
    }
    
    if content.is_empty() {
        content = "No README found".to_string();
    }
    Ok(content)
}

async fn process_dependencies(repo_path: &Path) -> Result<String> {
    let mut deps = String::new();
    let dep_files = ["package.json", "requirements.txt", "Cargo.toml", "go.mod"];
    let mut entries = tokio_fs::read_dir(repo_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            if let Some(fname) = entry.file_name().to_str() {
                if dep_files.iter().any(|&df| fname.to_lowercase() == df.to_lowercase()) {
                    deps.push_str(&format!("\n### {}\n", fname));
                    let content = tokio_fs::read_to_string(entry.path()).await?;
                    deps.push_str(&content);
                    deps.push_str("\n");
                }
            }
        }
    }
    
    if deps.is_empty() {
        deps = "No dependency files found".to_string();
    }
    Ok(deps)
}

async fn process_notebooks(repo_path: &Path) -> Result<String> {
    let mut notebooks = String::new();
    let mut entries = tokio_fs::read_dir(repo_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            if let Some(fname) = entry.file_name().to_str() {
                if NOTEBOOK_EXTENSIONS.iter().any(|&ext| fname.ends_with(ext)) {
                    notebooks.push_str(&format!("- {}\n", entry.path().display()));
                }
            }
        }
    }
    
    if notebooks.is_empty() {
        notebooks = "No Jupyter notebooks found".to_string();
    }
    Ok(notebooks)
}

/// Generates an index of the repository contents
pub async fn generate_repo_index(repo_path: &Path) -> Result<String> {
    let mut index = String::new();
    let mut entries = Vec::new();
    let mut dir = tokio_fs::read_dir(repo_path).await?;
    while let Some(entry) = dir.next_entry().await? {
        entries.push(entry);
    }
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if let Ok(content) = tokio_fs::read_to_string(&path).await {
            index.push_str(&format!("## {}\n\n", path.display()));
            index.push_str(&content);
            index.push_str("\n\n");
        }
    }

    Ok(index)
}

/// Downloads repository content from a given GitHub URL into the specified temporary directory.
/// It internally calls process_github_url and then searches the temporary directory for the generated file.
pub async fn process_github_content(github_url: &str, temp_dir: &Path) -> Result<String> {
    let pb = ProgressBar::hidden();
    process_github_url(github_url, temp_dir, &pb).await?;
    
    let mut entries = tokio_fs::read_dir(temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
            if name.contains("github_") {
                return tokio_fs::read_to_string(entry.path()).await
                    .map_err(ProcessorError::IO);
            }
        }
    }
    Err(ProcessorError::Processing("No GitHub content found".into()))
}

/// Downloads a file from the given URL to the specified output path.
pub async fn download_repo(owner: &str, repo: &str, output_path: &Path) -> Result<()> {
    let client = create_github_client()?;
    let url = format!("https://api.github.com/repos/{}/{}/zipball", owner, repo);
    
    let response = client
        .get(&url)
        .header("User-Agent", "llama-package-service")
        .send()
        .await
        .map_err(|e| ProcessorError::Http(e))?;

    if !response.status().is_success() {
        return Err(ProcessorError::Processing(format!(
            "Failed to download repository: {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| ProcessorError::Http(e))?;

    tokio_fs::write(output_path, bytes)
        .await
        .map_err(|e| ProcessorError::IO(e))?;

    Ok(())
}

/// Extracts a ZIP archive from the given path into the specified directory.
pub async fn extract_zip(zip_path: impl AsRef<Path>, extract_path: impl AsRef<Path>) -> Result<()> {
    let mut file = std_fs::File::open(zip_path.as_ref())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    let cursor = std::io::Cursor::new(buffer);
    let mut archive = zip::ZipArchive::new(cursor)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = extract_path.as_ref().join(file.mangled_name());
        
        if file.name().ends_with('/') {
            std_fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std_fs::create_dir_all(p)?;
                }
            }
            
            let _outfile = std_fs::File::create(&outpath)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            std_fs::write(&outpath, &buffer)?;
        }
    }
    
    Ok(())
}

/// Processor for GitHub repositories and organizations
///
/// This processor handles downloading, analyzing, and documenting
/// GitHub repositories and organizations.
pub struct GitHubProcessor {
    /// HTTP client for making GitHub API requests
    client: Client,
}

impl GitHubProcessor {
    /// Creates a new GitHub processor with default configuration
    ///
    /// # Returns
    /// A new GitHubProcessor instance
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }

    async fn validate_url(&self, url: &str) -> Result<()> {
        let parsed = Url::parse(url)
            .map_err(|_| ProcessorError::new("Invalid URL format"))?;
        
        if parsed.host_str() != Some("github.com") {
            return Err(ProcessorError::new("Not a GitHub URL"));
        }
        Ok(())
    }

    async fn extract_repo_info(&self, url: &str) -> Result<(String, String)> {
        let parsed = Url::parse(url)
            .map_err(|_| ProcessorError::new("Invalid URL format"))?;
            
        let segments: Vec<&str> = parsed.path_segments()
            .ok_or_else(|| ProcessorError::new("Invalid URL path"))?
            .collect();
            
        if segments.len() < 2 {
            return Err(ProcessorError::new("Invalid GitHub repository URL"));
        }
        
        Ok((segments[0].to_string(), segments[1].to_string()))
    }

    async fn get_repo_info(&self, owner: &str, repo: &str) -> Result<Value> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| ProcessorError::new(&format!("Failed to fetch repo info: {}", e)))?;
            
        response.json().await
            .map_err(|e| ProcessorError::new(&format!("Failed to parse repo info: {}", e)))
    }

    async fn download_repo(&self, owner: &str, repo: &str, branch: &str, output_dir: &Path) -> Result<PathBuf> {
        let url = format!("https://api.github.com/repos/{}/{}/zipball/{}", owner, repo, branch);
        let response = self.client.get(&url)
            .send()
            .await?;
        
        let bytes = response.bytes()
            .await
            .map_err(|e| ProcessorError::new(&format!("Failed to read response: {}", e)))?;
        
        let archive_path = output_dir.join("repo.zip");
        tokio_fs::write(&archive_path, &bytes).await?;
        
        let extract_path = output_dir.join("repo");
        tokio_fs::create_dir_all(&extract_path).await?;
        
        common_extract_archive(&bytes, &extract_path)?;
        
        let entries = list_directory_entries(&extract_path).await?;
        let root_dir = entries.into_iter()
            .next()
            .ok_or_else(|| ProcessorError::new("Empty archive"))?;
        
        Ok(root_dir)
    }

    async fn process_repo_contents(&self, repo_path: &Path, output_dir: &Path) -> Result<()> {
        let entries = list_directory_entries(repo_path).await?;
        
        let mut output = String::new();
        output.push_str("# Repository Analysis\n\n");
        
        output.push_str("## Files\n\n");
        for entry in entries {
            output.push_str(&format!("- {}\n", entry.display()));
        }
        
        let output_path = output_dir.join("repo_analysis.txt");
        tokio_fs::write(output_path, output).await?;
        
        Ok(())
    }

    async fn find_and_read_readme(&self, dir: &Path) -> Result<Option<String>> {
        let mut entries = tokio_fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().to_lowercase().contains("readme") {
                        let content = tokio_fs::read_to_string(path).await?;
                        return Ok(Some(content));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn process_dependencies(&self, repo_path: &Path) -> Result<String> {
        let mut deps = String::new();
        
        let mut entries = tokio_fs::read_dir(repo_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy();
                if filename == "package.json" || filename == "Cargo.toml" || filename == "requirements.txt" {
                    let content = tokio_fs::read_to_string(&path).await?;
                    deps.push_str(&format!("## {}\n\n```\n{}\n```\n\n", filename, content));
                }
            }
        }
        
        Ok(deps)
    }

    async fn generate_file_structure(&self, dir: &Path) -> Result<String> {
        let mut output = String::new();
        self.build_file_tree(dir, "", &mut output).await?;
        Ok(output)
    }

    async fn build_file_tree(&self, dir: &Path, prefix: &str, output: &mut String) -> Result<()> {
        let mut entries = tokio_fs::read_dir(dir).await?;
        let mut paths = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            paths.push(entry.path());
        }
        
        // Sort paths to make directories come first, then files alphabetically
        paths.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });
        
        for path in paths {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
                
            let new_prefix = format!("{}{}/", prefix, file_name);
            
            if path.is_dir() {
                output.push_str(&format!("{}\n", new_prefix));
                Box::pin(self.build_file_tree(&path, &new_prefix, output)).await?;
            } else {
                output.push_str(&format!("{}\n", new_prefix));
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl PackageProcessor for GitHubProcessor {
    fn name(&self) -> &'static str {
        "GitHub"
    }
    
    fn accepts(&self, url: &str) -> bool {
        url.contains("github.com")
    }
    
    async fn validate(&self, url: &str) -> Result<()> {
        if !url.contains("github.com") {
            return Err(ProcessorError::Message("Not a GitHub URL".to_string()));
        }
        Ok(())
    }
    
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        setup_github_progress_bar(&pb);
        
        // Extract repo details from URL
        let repo_details = extract_github_details(url)?;
        
        // Process directly without calling process_github_url
        process_github_repo_inner(&repo_details.owner, &repo_details.repo, output_dir, &pb).await?;
        
        pb.finish_with_message(format!("✨ GitHub repository {}/{} processed successfully", 
                                     repo_details.owner, repo_details.repo));
        Ok(())
    }
}

impl Clone for GitHubProcessor {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone()
        }
    }
}

// Create a GitHub client with appropriate headers
fn create_github_client() -> Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("llama-package-service"),
    );
    
    let client = Client::builder()
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .default_headers(headers)
        .build()
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
    
    Ok(client)
}

async fn make_github_request(client: &Client, url: &str) -> Result<Value> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(ProcessorError::GitHubApi(format!(
            "GitHub API request failed: HTTP {}",
            response.status()
        )));
    }
    Ok(response.json().await?)
}

/// Sets up a progress bar with GitHub-specific styling
pub fn setup_github_progress_bar(pb: &ProgressBar) {
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
}

// LLM function calling interface
#[cfg(feature = "llm")]
pub mod llm {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ProcessGitHubArgs {
        pub url: String,
        pub output_path: String,
    }

    pub async fn process_github(args: ProcessGitHubArgs) -> std::result::Result<String, String> {
        let pb = ProgressBar::new_spinner();
        setup_github_progress_bar(&pb);
        
        let output_dir = PathBuf::from(args.output_path);
        
        match process_github_url(&args.url, &output_dir, &pb).await {
            Ok(_) => Ok("Successfully processed GitHub repository".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use tokio;

    #[tokio::test]
    async fn test_github_api_requests() {
        let mut mock_server = mockito::Server::new_async().await;
        
        // Override GitHub API base URL to point to the mock server
        std::env::set_var("GITHUB_API_BASE_URL", mock_server.url());
        
        // Mock a repository request
        let mock_repo = mock_server.mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name": "repo", "description": "Test repo"}"#)
            .create();

        let client_result = create_github_client();
        assert!(client_result.is_ok());
        let client = client_result.unwrap();
        
        let result = fetch_repo_info(&client, "owner", "repo").await;
        assert!(result.is_ok());
        
        mock_repo.assert();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    #[ignore]
    async fn test_process_github_repo() {
        let temp_dir = TempDir::new().unwrap();
        let pb = ProgressBar::new(100);
        
        // Test with a known public repository
        let result = process_github_repo(
            "https://github.com/ultrafunkamsterdam/nodriver",
            temp_dir.path(),
            &pb
        ).await;
        
        assert!(result.is_ok());
    }
}

// MLX Integration types for structured output
/// Analysis of code repositories with various metrics and statistics
///
/// This struct contains analytical information about a code repository,
/// including language statistics, dependencies, complexity metrics, and
/// a semantic summary of the codebase.
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeAnalysis {
    /// Statistics about languages used in the repository
    pub language_stats: HashMap<String, usize>,
    /// Key dependencies used in the project
    pub key_dependencies: Vec<String>,
    /// Complexity metrics for the code
    pub complexity_metrics: ComplexityMetrics,
    /// Semantic summary of the codebase
    pub semantic_summary: String,
}

/// Metrics for evaluating code complexity and maintainability
///
/// This struct contains various metrics that evaluate the complexity
/// and maintainability of a codebase, which can be used to assess
/// its quality and potential development challenges.
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity score (higher means more complex)
    pub cyclomatic_complexity: f32,
    /// Cognitive complexity score (higher means harder to understand)
    pub cognitive_complexity: f32,
    /// Maintainability index (higher is better)
    pub maintainability_index: f32,
}

/// Download a GitHub repository content
pub async fn download_repository(repo_url: &str) -> Result<String> {
    let temp_dir = tempdir().map_err(|e| ProcessorError::IO(e))?;
    let _pb = ProgressBar::new_spinner();
    
    // Extract owner and repo from the URL
    let url_type = determine_github_url_type(repo_url).await?;
    
    match url_type {
        GitHubUrlType::Repository { owner, repo } => {
            // Create client
            let client = create_github_client()?;
            
            // Fetch repo info
            let repo_info = fetch_repo_info(&client, &owner, &repo).await?;
            let default_branch = repo_info["default_branch"].as_str().unwrap_or("main");
            
            // Download the repo archive
            let archive_bytes = download_repo_archive(&client, &owner, &repo, default_branch).await?;
            
            // Extract the archive
            extract_archive(&archive_bytes, temp_dir.path()).await?;
            
            // Process the content
            process_github_content(repo_url, temp_dir.path()).await
        },
        GitHubUrlType::Organization { org: _ } => {
            Err(ProcessorError::Message("Organization URLs are not supported for direct download".to_string()))
        }
    }
}

/// Process repository content to extract useful information
pub fn process_repository_content(content: &str) -> Result<String> {
    let mut processed_content = String::new();
    
    // Add header with timestamp
    processed_content.push_str(&format!("# Repository Analysis\n"));
    processed_content.push_str(&format!("Analysis Date: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
    
    // Add the content
    processed_content.push_str(&format!("## Repository Content\n\n"));
    
    // Extract code sections
    let _code_sections = extract_code_sections(content);
    if !_code_sections.is_empty() {
        processed_content.push_str("## Code Sections\n\n");
        processed_content.push_str(&_code_sections);
    }
    
    // Extract README sections
    let readme_sections = extract_readme_sections(content);
    if !readme_sections.is_empty() {
        processed_content.push_str("\n## README Content\n\n");
        processed_content.push_str(&readme_sections);
    }
    
    // Extract dependency information
    let dependency_info = extract_dependency_info(content);
    if !dependency_info.is_empty() {
        processed_content.push_str("\n## Dependencies\n\n");
        processed_content.push_str(&dependency_info);
    }
    
    Ok(processed_content)
}

/// Extract code sections from the content
fn extract_code_sections(content: &str) -> String {
    let mut result = String::new();
    
    // Look for common code patterns
    for line in content.lines() {
        if line.contains("```") || line.trim().starts_with("def ") || 
           line.trim().starts_with("func ") || line.trim().starts_with("function ") ||
           line.trim().starts_with("class ") || line.trim().starts_with("struct ") ||
           line.trim().starts_with("impl ") || line.trim().starts_with("trait ") {
            result.push_str(line);
            result.push('\n');
        }
    }
    
    result
}

/// Extract README sections from the content
fn extract_readme_sections(content: &str) -> String {
    let mut result = String::new();
    let mut in_readme = false;
    
    for line in content.lines() {
        if line.contains("README") || line.contains("# ") {
            in_readme = true;
        }
        
        if in_readme && (line.starts_with('#') || line.contains("Usage") || 
                         line.contains("Installation") || line.contains("Features") ||
                         line.contains("Overview")) {
            result.push_str(line);
            result.push('\n');
        }
    }
    
    result
}

/// Extract dependency information from the content
fn extract_dependency_info(content: &str) -> String {
    let mut result = String::new();
    
    // Look for package.json, Cargo.toml, requirements.txt, etc.
    if content.contains("package.json") || content.contains("\"dependencies\":") {
        result.push_str("Node.js dependencies found\n");
    }
    
    if content.contains("Cargo.toml") || content.contains("[dependencies]") {
        result.push_str("Rust dependencies found\n");
    }
    
    if content.contains("requirements.txt") || content.contains("setup.py") {
        result.push_str("Python dependencies found\n");
    }
    
    if content.contains("build.gradle") || content.contains("pom.xml") {
        result.push_str("Java dependencies found\n");
    }
    
    result
}

async fn is_github_main_file(entry_path: PathBuf) -> bool {
    // Check if the file is a main file in a GitHub repository
    if let Some(file_name) = entry_path.file_name().and_then(|n| n.to_str()) {
        return file_name.contains("README") || 
               file_name.contains("LICENSE") || 
               file_name == "package.json" || 
               file_name == "go.mod" || 
               file_name == "Cargo.toml";
    }
    false
}

async fn list_directory_entries(dir_path: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    let mut dir_stream = tokio_fs::read_dir(dir_path).await?;
    
    while let Some(entry) = dir_stream.next_entry().await? {
        entries.push(entry.path());
    }
    
    Ok(entries)
}

async fn find_main_repository_dir(extract_path: &Path) -> Result<PathBuf> {
    // Use the list_directory_entries function
    let entries = list_directory_entries(extract_path).await?;
    
    // Now we can use normal iterators since we've already collected the entries
    for path in entries {
        if path.is_dir() {
            // Look for the main directory (usually has the repository name in it)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains("-main") || name.contains("-master") {
                    return Ok(path);
                }
            }
        }
    }
    
    // If we didn't find a main directory, just return the extract path
    Ok(extract_path.to_path_buf())
}

async fn generate_repo_structure(repo_path: &Path) -> Result<String> {
    let entries = list_directory_entries(repo_path).await?;
    
    let mut output = String::new();
    output.push_str("# Repository Structure\n\n");
    
    for path in entries {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            output.push_str(&format!("- {}\n", name));
            
            if path.is_dir() {
                // Add subdirectories
                let subentries = list_directory_entries(&path).await?;
                for subpath in subentries {
                    if let Some(subname) = subpath.file_name().and_then(|n| n.to_str()) {
                        output.push_str(&format!("  - {}\n", subname));
                    }
                }
            }
        }
    }
    
    Ok(output)
}

async fn extract_main_dir(bytes: &[u8], output_dir: &Path) -> Result<PathBuf> {
    let extract_path = output_dir.join("repo");
    tokio_fs::create_dir_all(&extract_path).await?;
    
    common_extract_archive(&bytes, &extract_path)?;
    
    // Use the find_main_repository_dir function instead
    find_main_repository_dir(&extract_path).await
}

async fn process_repo_contents(repo_path: &Path, output_dir: &Path) -> Result<()> {
    // Use the list_directory_entries function 
    let entries = list_directory_entries(repo_path).await?;
    
    let mut output = String::new();
    output.push_str("# Repository Analysis\n\n");
    
    output.push_str("## Files\n\n");
    for entry_path in entries {
        output.push_str(&format!("- {}\n", entry_path.display()));
    }
    
    let output_path = output_dir.join("repo_analysis.txt");
    tokio_fs::write(output_path, output).await?;
    
    Ok(())
}

async fn generate_repository_overview(
    client: &Client,
    owner: &str,
    repo: &str,
) -> Result<String> {
    let mut content = String::new();
    content.push_str(&format!("# Repository Overview: {}/{}\n\n", owner, repo));

    // Add a link to the repository
    content.push_str(&format!("URL: https://github.com/{}/{}\n\n", owner, repo));

    // Fetch repository details using the async function
    let repo_details = fetch_repo_info(client, owner, repo).await?;
    
    // Safely access topics from the Value object
    if let Some(repo_obj) = repo_details.as_object() {
        if let Some(topics_value) = repo_obj.get("topics") {
            if let Some(topics) = topics_value.as_array() {
                content.push_str("\n## Topics\n");
                for topic in topics {
                    if let Some(topic_str) = topic.as_str() {
                        content.push_str(&format!("- {}\n", topic_str));
                    }
                }
            }
        }
    }

    Ok(content)
}

// Add structure for repo details
struct GitHubRepoDetails {
    owner: String,
    repo: String,
}

// Add helper function to extract repo details
fn extract_github_details(url: &str) -> Result<GitHubRepoDetails> {
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 5 {
        return Err(ProcessorError::Validation("Invalid GitHub URL format".to_string()));
    }
    
    Ok(GitHubRepoDetails {
        owner: parts[3].to_string(),
        repo: parts[4].to_string(),
    })
}

// Add the actual processing function that doesn't create more processors
async fn process_github_repo_inner(owner: &str, repo: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    // Use the progress bar
    pb.set_message(format!("Processing GitHub repository {}/{}", owner, repo));
    
    // Create client
    let client = create_github_client()?;
    
    // Fetch repo info
    pb.set_message(format!("Fetching repository info for {}/{}", owner, repo));
    let repo_info = fetch_repo_info(&client, owner, repo).await?;
    let default_branch = repo_info["default_branch"].as_str().unwrap_or("main");
    
    // Create repo directory in output_dir
    let repo_dir = output_dir.join(format!("github_{}_{}_{}", owner, repo, chrono::Utc::now().format("%Y%m%d%H%M%S")));
    tokio_fs::create_dir_all(&repo_dir).await?;
    
    // Download the repo archive
    pb.set_message(format!("Downloading {}/{} (branch: {})", owner, repo, default_branch));
    let archive_bytes = download_repo_archive(&client, owner, repo, default_branch).await?;
    
    // Create a temp directory for extraction
    let extract_dir = repo_dir.join("repo_content");
    tokio_fs::create_dir_all(&extract_dir).await?;
    
    // Extract the archive
    pb.set_message("Extracting repository contents");
    extract_archive(&archive_bytes, &extract_dir).await?;
    
    // Find the root directory in the extracted content (usually there's one top-level directory)
    let mut entries = tokio_fs::read_dir(&extract_dir).await?;
    let mut root_dir = None;
    
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            root_dir = Some(entry.path());
            break;
        }
    }
    
    let root_dir = match root_dir {
        Some(dir) => dir,
        None => return Err(ProcessorError::Processing("No repository content found".to_string())),
    };
    
    // Initialize content string
    let mut content = String::new();
    
    // Generate repo overview
    pb.set_message("Generating repository overview");
    content.push_str(&format!("## Repository Information\n\n"));
    content.push_str(&format!("- **Repository:** {}/{}\n", owner, repo));
    content.push_str(&format!("- **URL:** https://github.com/{}/{}\n", owner, repo));
    content.push_str(&format!("- **Processed:** {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    
    // Add repository details from the API info
    if let Some(description) = repo_info["description"].as_str() {
        content.push_str(&format!("- **Description:** {}\n\n", description));
    }
    
    content.push_str(&format!("- **Default Branch:** {}\n", default_branch));
    content.push_str(&format!("- **Stars:** {}\n", repo_info["stargazers_count"].as_u64().unwrap_or(0)));
    content.push_str(&format!("- **Forks:** {}\n", repo_info["forks_count"].as_u64().unwrap_or(0)));
    content.push_str(&format!("- **Issues:** {}\n", repo_info["open_issues_count"].as_u64().unwrap_or(0)));
    content.push_str(&format!("- **Last Updated:** {}\n", repo_info["updated_at"].as_str().unwrap_or("Unknown")));
    
    if let Some(language) = repo_info["language"].as_str() {
        content.push_str(&format!("- **Primary Language:** {}\n", language));
    }
    
    // Add license information if available
    if let Some(license) = repo_info["license"].as_object() {
        if let Some(name) = license.get("name").and_then(|n| n.as_str()) {
            content.push_str(&format!("- **License:** {}\n", name));
        }
    }
    
    // Add topics/tags if available
    if let Some(topics) = repo_info["topics"].as_array() {
        if !topics.is_empty() {
            content.push_str("\n### Topics\n\n");
            for topic in topics {
                if let Some(topic_str) = topic.as_str() {
                    content.push_str(&format!("- {}\n", topic_str));
                }
            }
        }
    }
    
    // Generate file tree
    content.push_str("\n## Repository Structure\n\n```\n");
    let mut file_tree = String::new();
    build_file_tree(&root_dir, "", &mut file_tree).await?;
    content.push_str(&file_tree);
    content.push_str("```\n\n");
    
    // Look for README file
    pb.set_message("Processing README file");
    content.push_str("## README Content\n\n");
    let readme = find_and_read_file(&root_dir, "README.md").await;
    if let Ok(readme_content) = readme {
        content.push_str(&readme_content);
    } else {
        // Try README.txt, README.rst, or just README
        let readme_txt = find_and_read_file(&root_dir, "README.txt").await;
        if let Ok(readme_txt_content) = readme_txt {
            content.push_str(&readme_txt_content);
        } else {
            let readme_rst = find_and_read_file(&root_dir, "README.rst").await;
            if let Ok(readme_rst_content) = readme_rst {
                content.push_str(&readme_rst_content);
            } else {
                let readme_no_ext = find_and_read_file(&root_dir, "README").await;
                if let Ok(readme_no_ext_content) = readme_no_ext {
                    content.push_str(&readme_no_ext_content);
                } else {
                    content.push_str("No README found in repository.\n");
                }
            }
        }
    }
    content.push_str("\n\n");
    
    // Look for package files
    pb.set_message("Processing dependency files");
    content.push_str("## Dependencies\n\n");
    let mut found_deps = false;
    for package_file in &["package.json", "Cargo.toml", "pyproject.toml", "requirements.txt", "go.mod", "pom.xml", "build.gradle", "Gemfile"] {
        if let Ok(pkg_content) = find_and_read_file(&root_dir, package_file).await {
            content.push_str(&format!("### Package File: {}\n\n```\n", package_file));
            content.push_str(&pkg_content);
            content.push_str("\n```\n\n");
            found_deps = true;
        }
    }
    if !found_deps {
        content.push_str("No dependency files found.\n\n");
    }
    
    // Process code files
    pb.set_message("Processing code files");
    content.push_str("## Code Files\n\n");
    
    // Find all code files
    let mut code_files = Vec::new();
    find_code_files(&root_dir, &mut code_files).await?;
    
    if code_files.is_empty() {
        content.push_str("No code files found in the repository.\n\n");
    } else {
        // Sort code files by path
        code_files.sort();
        
        // Count extensions to report language distribution
        let mut extension_counts = std::collections::HashMap::new();
        for file_path in &code_files {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                *extension_counts.entry(ext.to_lowercase()).or_insert(0) += 1;
            }
        }
        
        // Report language distribution
        if !extension_counts.is_empty() {
            content.push_str("### Language Distribution\n\n");
            let total = code_files.len();
            let mut sorted_exts: Vec<_> = extension_counts.iter().collect();
            sorted_exts.sort_by(|a, b| b.1.cmp(a.1));
            
            for (ext, count) in sorted_exts {
                let percentage = (*count as f64 / total as f64) * 100.0;
                content.push_str(&format!("- **{}**: {} files ({:.1}%)\n", ext, count, percentage));
            }
            content.push_str("\n");
        }
        
        // Add code size analysis
        if let Ok(size_analysis) = calculate_code_size_metrics(&root_dir).await {
            content.push_str(&size_analysis);
        }
        
        // Include the full set of code files (no artificial limit)
        let important_files = code_files.clone();
        
        content.push_str("### Key Source Files\n\n");
        
        // Group files by directory for better organization
        let mut files_by_dir = std::collections::HashMap::new();
        for file_path in important_files {
            let parent = file_path.parent().unwrap_or(Path::new(""));
            let rel_parent = parent.strip_prefix(&extract_dir).unwrap_or(parent);
            let entry = files_by_dir.entry(rel_parent.to_path_buf()).or_insert_with(Vec::new);
            entry.push(file_path);
        }
        
        // Sort directories for consistent output
        let mut dirs: Vec<_> = files_by_dir.keys().collect();
        dirs.sort();
        
        // Process each directory
        for dir in dirs {
            if let Some(files) = files_by_dir.get(dir) {
                let dir_name = dir.to_string_lossy();
                if !dir_name.is_empty() {
                    content.push_str(&format!("#### Directory: `{}`\n\n", dir_name));
                }
                
                for file_path in files {
                    let rel_path = file_path.strip_prefix(&extract_dir)
                        .unwrap_or(&file_path)
                        .to_string_lossy();
                    
                    content.push_str(&format!("##### File: `{}`\n\n", rel_path));
                    
                    if let Ok(file_content) = tokio::fs::read_to_string(&file_path).await {
                        // Determine language for syntax highlighting
                        let extension = file_path.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("");
                            
                        let language = match extension {
                            "js" => "javascript",
                            "ts" => "typescript",
                            "py" => "python",
                            "rs" => "rust",
                            "go" => "go",
                            "java" => "java",
                            "c" | "cpp" | "h" | "hpp" => "cpp",
                            "cs" => "csharp",
                            "rb" => "ruby",
                            "php" => "php",
                            "swift" => "swift",
                            "kt" => "kotlin",
                            "md" => "markdown",
                            "json" => "json",
                            "yml" | "yaml" => "yaml",
                            "toml" => "toml",
                            _ => "",
                        };
                        
                        content.push_str(&format!("```{}\n", language));
                        content.push_str(&file_content);
                        content.push_str("\n```\n\n");
                    } else {
                        content.push_str("*Unable to read file content*\n\n");
                    }
                }
            }
        }
    }
    
    // Look for documentation
    pb.set_message("Processing documentation");
    content.push_str("## Documentation\n\n");
    
    // Check for documentation in common locations
    let doc_patterns = [
        "docs/", "doc/", "documentation/", "wiki/", "guide/",
        "CONTRIBUTING.md", "CHANGELOG.md", "DEVELOPMENT.md", "API.md",
        "INSTALL.md", "USAGE.md", "CODE_OF_CONDUCT.md"
    ];
    
    let mut found_docs = false;
    for pattern in &doc_patterns {
        if pattern.ends_with('/') {
            // This is a directory pattern
            let dir_pattern = pattern.trim_end_matches('/');
            let docs_path = root_dir.join(dir_pattern);
            if docs_path.exists() && docs_path.is_dir() {
                content.push_str(&format!("### Documentation Directory: {}\n\n", dir_pattern));
                
                // List doc files
                let mut doc_files = Vec::new();
                if let Ok(entries) = tokio_fs::read_dir(&docs_path).await {
                    let mut entries = entries;
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                if ext == "md" || ext == "rst" || ext == "txt" {
                                    doc_files.push(path);
                                }
                            }
                        }
                    }
                }
                
                if !doc_files.is_empty() {
                    found_docs = true;
                    doc_files.sort();
                    
                    // Take up to 5 doc files
                    for doc_file in doc_files.iter().take(5) {
                        let file_name = doc_file.file_name().unwrap_or_default().to_string_lossy();
                        content.push_str(&format!("#### {}\n\n", file_name));
                        
                        if let Ok(doc_content) = tokio::fs::read_to_string(&doc_file).await {
                            content.push_str(&doc_content);
                            content.push_str("\n\n");
                        }
                    }
                }
            }
        } else {
            // This is a specific file pattern
            let doc_path = root_dir.join(pattern);
            if doc_path.exists() && doc_path.is_file() {
                found_docs = true;
                content.push_str(&format!("### {}\n\n", pattern));
                
                if let Ok(doc_content) = tokio::fs::read_to_string(&doc_path).await {
                    content.push_str(&doc_content);
                    content.push_str("\n\n");
                }
            }
        }
    }
    
    if !found_docs {
        content.push_str("No additional documentation found.\n\n");
    }
    
    // Organize and enhance the content
    let organized_content = common::organize_content(&content, &format!("{}/{}", owner, repo), "github");
    
    // Save comprehensive output
    let output_path = common::save_comprehensive_output(
        &format!("{}-{}", owner, repo),
        "github",
        &organized_content,
        output_dir,
        pb
    ).await?;
    
    // Also save a copy to the main output directory with a standardized name format
    let main_output_filename = format!("{}-{}_github_repo.txt", owner, repo);
    let main_output_path = output_dir.join(&main_output_filename);
    tokio::fs::write(&main_output_path, &organized_content).await?;
    
    pb.finish_with_message(format!("[SUCCESS] Repository {}/{} processed successfully. Output saved to: {}", 
                                 owner, repo, output_path.display()));
    Ok(())
}

// Helper function to build a file tree (fix recursion with Box::pin)
async fn build_file_tree(dir: &Path, prefix: &str, output: &mut String) -> Result<()> {
    let mut entries = tokio_fs::read_dir(dir).await?;
    let mut paths = Vec::new();
    
    while let Some(entry) = entries.next_entry().await? {
        paths.push(entry.path());
    }
    
    // Sort paths to make directories come first, then files alphabetically
    paths.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();
        
        if a_is_dir && !b_is_dir {
            std::cmp::Ordering::Less
        } else if !a_is_dir && b_is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.file_name().cmp(&b.file_name())
        }
    });
    
    for path in paths {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
            
        if path.is_dir() {
            output.push_str(&format!("{}└── {}/\n", prefix, file_name));
            // Use Box::pin to handle recursion in async function
            Box::pin(build_file_tree(&path, &format!("{}    ", prefix), output)).await?;
        } else {
            output.push_str(&format!("{}└── {}\n", prefix, file_name));
        }
    }
    
    Ok(())
}

// Helper function to find and read a file in a directory (case-insensitive)
async fn find_and_read_file(dir: &Path, filename: &str) -> Result<String> {
    let lower_filename = filename.to_lowercase();
    let mut entries = tokio_fs::read_dir(dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.to_lowercase() == lower_filename {
                    return tokio_fs::read_to_string(&path).await.map_err(|e| ProcessorError::IO(e));
                }
            }
        } else if path.is_dir() {
            // Try in subdirectories, using Box::pin to handle recursion
            match Box::pin(find_and_read_file(&path, filename)).await {
                Ok(content) => return Ok(content),
                Err(_) => continue, // Continue searching if not found in this directory
            }
        }
    }
    
    Err(ProcessorError::Message(format!("File {} not found", filename)))
}

/// Find all code files in a directory recursively
async fn find_code_files(dir: &Path, result: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        let mut entries = tokio_fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Skip hidden directories and common directories to ignore
                let dir_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                    
                if dir_name.starts_with(".") || 
                   ["node_modules", "target", "build", "dist", "venv", 
                    "__pycache__", ".git", ".github"].contains(&dir_name) {
                    continue;
                }
                
                // Use Box::pin to handle recursion in async function
                Box::pin(find_code_files(&path, result)).await?;
            } else if path.is_file() {
                // Check if it's a code file based on extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if CODE_EXTENSIONS.contains(&format!(".{}", ext).as_str()) {
                        result.push(path);
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Select important files from a list of files
fn select_important_files(files: &[PathBuf], max_files: usize) -> Vec<PathBuf> {
    if files.len() <= max_files {
        return files.to_vec();
    }
    
    // Priority patterns for important files - ordering matters
    let priority_patterns = [
        // Entry points
        "main.rs", "main.go", "main.py", "index.js", "index.ts", "app.js", "app.ts", 
        "server.js", "server.ts", "app.py", "main.cpp", "main.c",
        // Configuration files
        "Cargo.toml", "package.json", "requirements.txt", "go.mod", "Makefile", 
        "CMakeLists.txt", "Dockerfile", "docker-compose.yml", ".github/workflows",
        // Documentation
        "README", "LICENSE", "CONTRIBUTING", "CHANGELOG", "API.md", "docs/",
        // Core modules
        "src/", "lib/", "core/", "internal/", "pkg/", "utils/", "helpers/",
        // Tests
        "tests/", "test_", "_test.go", "spec.js", "spec.ts", ".test.js", ".test.ts", 
        "pytest", "unittest"
    ];
    
    // Score each file based on priority patterns, path depth, and file size
    let mut scored_files: Vec<(i32, &PathBuf)> = files.iter()
        .map(|file| {
            let path_str = file.to_string_lossy();
            
            // Base score - prefer shorter paths (closer to root)
            let depth_score = -(path_str.matches('/').count() as i32);
            
            // Priority score based on patterns 
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
    
    // Ensure diverse file selection by taking files from different directories
    let mut selected_files = Vec::new();
    let mut selected_dirs = std::collections::HashSet::new();
    
    // First take high-priority files up to half of max_files
    for (_, file) in scored_files.iter().take(max_files / 2) {
        selected_files.push((*file).clone());
        if let Some(parent) = file.parent() {
            selected_dirs.insert(parent.to_path_buf());
        }
    }
    
    // Then take remaining files, prioritizing those from directories not yet represented
    for (_, file) in scored_files.iter().skip(max_files / 2) {
        if selected_files.len() >= max_files {
            break;
        }
        
        if let Some(parent) = file.parent() {
            if !selected_dirs.contains(parent) {
                selected_files.push((*file).clone());
                selected_dirs.insert(parent.to_path_buf());
            }
        } else if selected_files.len() < max_files {
            selected_files.push((*file).clone());
        }
    }
    
    // If we haven't filled our quota, add more files from the high-scoring ones
    if selected_files.len() < max_files {
        for (_, file) in scored_files.iter() {
            if selected_files.len() >= max_files || selected_files.contains(*file) {
                continue;
            }
            selected_files.push((*file).clone());
        }
    }
    
    selected_files
}

/// Calculate code size metrics for a repository
async fn calculate_code_size_metrics(root_dir: &Path) -> Result<String> {
    let mut result = String::new();
    result.push_str("## Code Size Analysis\n\n");
    
    // Collect all code files
    let mut all_files = Vec::new();
    find_all_files(root_dir, &mut all_files).await?;
    
    // Filter for code files only
    let code_files: Vec<_> = all_files.into_iter()
        .filter(|path| {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                CODE_EXTENSIONS.iter().any(|&code_ext| 
                    code_ext.trim_start_matches('.') == ext)
            } else {
                false
            }
        })
        .collect();
    
    if code_files.is_empty() {
        result.push_str("No code files found for analysis.\n\n");
        return Ok(result);
    }
    
    // Group by language/extension
    let mut lang_files: std::collections::HashMap<String, Vec<PathBuf>> = std::collections::HashMap::new();
    let mut lang_sizes: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut lang_loc: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_size: u64 = 0;
    let mut total_loc: usize = 0;
    
    for file_path in &code_files {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            let lang = ext.to_lowercase();
            lang_files.entry(lang.clone()).or_default().push(file_path.clone());
            
            // Get file size
            if let Ok(metadata) = std::fs::metadata(file_path) {
                let size = metadata.len();
                *lang_sizes.entry(lang.clone()).or_default() += size;
                total_size += size;
                
                // Count lines of code
                if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                    let loc = content.lines().count();
                    *lang_loc.entry(lang.clone()).or_default() += loc;
                    total_loc += loc;
                }
            }
        }
    }
    
    // Display metrics by language
    result.push_str("| Language | Files | Lines of Code | Size (KB) | % of Codebase |\n");
    result.push_str("|----------|-------|---------------|-----------|---------------|\n");
    
    let mut langs: Vec<_> = lang_files.keys().collect();
    langs.sort();
    
    for lang in langs {
        let files = lang_files.get(lang).unwrap();
        let size = lang_sizes.get(lang).unwrap_or(&0);
        let lines = lang_loc.get(lang).unwrap_or(&0);
        let percentage = if total_size > 0 { 
            (*size as f64 / total_size as f64) * 100.0 
        } else { 
            0.0 
        };
        
        result.push_str(&format!("| {} | {} | {} | {:.2} | {:.1}% |\n", 
            lang, files.len(), lines, (*size as f64) / 1024.0, percentage));
    }
    
    // Add totals
    result.push_str(&format!("| **Total** | **{}** | **{}** | **{:.2}** | **100%** |\n\n", 
        code_files.len(), total_loc, (total_size as f64) / 1024.0));
        
    Ok(result)
}

/// Recursively find all files in a directory
async fn find_all_files(dir: &Path, result: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        let mut entries = tokio_fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Skip hidden directories and common directories to ignore
                let dir_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                    
                if dir_name.starts_with(".") || 
                   ["node_modules", "target", "build", "dist", "venv", 
                    "__pycache__", ".git", ".github"].contains(&dir_name) {
                    continue;
                }
                
                Box::pin(find_all_files(&path, result)).await?;
            } else if path.is_file() {
                result.push(path);
            }
        }
    }
    
    Ok(())
}

/// Process a GitHub repository URL
///
/// This is a public wrapper function for testing purposes that processes a GitHub repository
/// by extracting the owner and repo name from the URL and processing it.
pub async fn process_github_repo(url: &str, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
    // Extract owner and repo from URL
    let repo_details = extract_github_details(url)?;
    
    // Process the repository
    process_github_repo_inner(&repo_details.owner, &repo_details.repo, output_dir, pb).await
}

// ... existing code ...
/// Returns the base URL for GitHub API, overridable via the `GITHUB_API_BASE_URL` env var (useful for testing).
fn github_api_base() -> String {
    std::env::var("GITHUB_API_BASE_URL").unwrap_or_else(|_| GITHUB_API_BASE.to_string())
}
