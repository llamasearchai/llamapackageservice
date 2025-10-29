#!/bin/bash
set -e

echo "🔧 Fixing GitHub processor..."

cat > src/processors/github.rs << 'EOL'
use std::path::{Path, PathBuf};
use tokio::fs as tokio_fs;
use std::fs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use indicatif::ProgressBar;

use crate::error::{ProcessorError, Result};
use crate::processors::PackageProcessor;
use crate::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub description: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub default_branch: String,
}

pub struct GitHubProcessor {
    client: Client,
}

impl GitHubProcessor {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn download_repo(&self, owner: &str, repo: &str, output_dir: &Path) -> Result<()> {
        // Create output directory
        tokio_fs::create_dir_all(output_dir).await?;
        
        // Get repo info
        let repo_info = self.fetch_repo_info(owner, repo).await?;
        
        // Download zip archive
        let zip_url = format!("https://github.com/{}/{}/archive/refs/heads/{}.zip", 
            owner, repo, repo_info.default_branch);
            
        let zip_path = output_dir.join("repo.zip");
        self.download_zip(&zip_url, &zip_path).await?;
        
        // Extract zip
        let extract_path = output_dir.join("extract");
        self.extract_zip(&zip_path, &extract_path).await?;
        
        // Generate overview
        self.generate_overview(owner, repo, &repo_info, output_dir).await?;
        
        // Generate file tree
        self.generate_file_tree(&extract_path, output_dir).await?;
        
        // Extract source files
        self.extract_source_files(&extract_path, output_dir).await?;
        
        Ok(())
    }

    async fn download_zip(&self, url: &str, output_path: &Path) -> Result<()> {
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Download(
                format!("Failed to download zip: HTTP {}", response.status())
            ));
        }
        
        let bytes = response.bytes()
            .await
            .map_err(|e| ProcessorError::Network(e))?;
            
        tokio_fs::write(output_path, bytes).await?;
        
        Ok(())
    }

    async fn extract_zip(&self, zip_path: &Path, extract_dir: &Path) -> Result<()> {
        tokio_fs::create_dir_all(extract_dir).await?;
        
        // Use std::fs for zip extraction as zip-rs doesn't have async support
        let file = fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_dir.join(file.name());
            
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

    async fn fetch_repo_info(&self, owner: &str, repo: &str) -> Result<RepoInfo> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        
        let response = self.client.get(&url)
            .header("User-Agent", "code2txt")
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Api(
                format!("GitHub API returned error status: {}", response.status())
            ));
        }
        
        response.json::<RepoInfo>()
            .await
            .map_err(|e| ProcessorError::Parsing(format!("Failed to parse repo info: {}", e)))
    }

    async fn generate_overview(&self, owner: &str, repo: &str, repo_info: &RepoInfo, output_dir: &Path) -> Result<()> {
        let mut overview = String::new();
        
        overview.push_str(&format!("# {}\n\n", repo));
        
        if let Some(description) = &repo_info.description {
            overview.push_str(&format!("{}\n\n", description));
        }
        
        overview.push_str(&format!("- Stars: {}\n", repo_info.stargazers_count));
        overview.push_str(&format!("- Forks: {}\n", repo_info.forks_count));
        overview.push_str(&format!("- Repository: https://github.com/{}/{}\n", owner, repo));
        
        // Get topics
        let topics = self.fetch_repo_topics(owner, repo).await?;
        if !topics.is_empty() {
            overview.push_str("\n## Topics\n\n");
            for topic in topics {
                overview.push_str(&format!("- {}\n", topic));
            }
        }
        
        // Save overview
        let overview_path = output_dir.join("overview.md");
        tokio_fs::write(&overview_path, overview).await?;
        
        Ok(())
    }

    async fn fetch_repo_topics(&self, owner: &str, repo: &str) -> Result<Vec<String>> {
        let url = format!("https://api.github.com/repos/{}/{}/topics", owner, repo);
        
        let response = self.client.get(&url)
            .header("User-Agent", "code2txt")
            .header("Accept", "application/vnd.github.mercy-preview+json")
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Api(
                format!("GitHub API returned error status: {}", response.status())
            ));
        }
        
        #[derive(Deserialize)]
        struct TopicsResponse {
            topics: Vec<String>,
        }
        
        let topics_response = response.json::<TopicsResponse>()
            .await
            .map_err(|e| ProcessorError::Parsing(format!("Failed to parse topics: {}", e)))?;
            
        Ok(topics_response.topics)
    }

    async fn generate_file_tree(&self, extract_path: &Path, output_dir: &Path) -> Result<()> {
        // Find the root directory (usually there's only one directory in the extract path)
        let mut entries = Vec::new();
        let mut readdir = tokio_fs::read_dir(extract_path).await?;
        
        while let Some(entry) = readdir.next_entry().await? {
            entries.push(entry);
        }
        
        if entries.is_empty() {
            return Err(ProcessorError::Processing("Empty repository".to_string()));
        }
        
        let root_dir_entry = &entries[0];
        if !root_dir_entry.file_type().await?.is_dir() {
            return Err(ProcessorError::Processing("Invalid repository structure".to_string()));
        }
        
        let root_path = root_dir_entry.path();
        
        // Generate file tree
        let mut tree = String::new();
        tree.push_str(&format!("{}\n", root_path.file_name().unwrap().to_string_lossy()));
        self.build_file_tree(&root_path, "", &mut tree).await?;
        
        // Save file tree
        let tree_path = output_dir.join("file_tree.txt");
        tokio_fs::write(&tree_path, tree).await?;
        
        Ok(())
    }

    async fn build_file_tree(&self, dir: &Path, prefix: &str, output: &mut String) -> Result<()> {
        let mut entries = Vec::new();
        let mut readdir = tokio_fs::read_dir(dir).await?;
        
        while let Some(entry) = readdir.next_entry().await? {
            entries.push(entry);
        }
            
        // Sort entries by name
        entries.sort_by(|a, b| {
            let a_name = a.file_name().to_string_lossy();
            let b_name = b.file_name().to_string_lossy();
            a_name.cmp(&b_name)
        });
        
        let count = entries.len();
        for (i, entry) in entries.into_iter().enumerate() {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            
            let is_last = i == count - 1;
            let marker = if is_last { "└── " } else { "├── " };
            let sub_prefix = if is_last { "    " } else { "│   " };
            
            let name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy();
                
            // Skip hidden files/directories
            if name.starts_with('.') {
                continue;
            }
            
            output.push_str(&format!("{}{}{}",
                prefix, marker, name));
            output.push('\n');
            
            if file_type.is_dir() {
                let new_prefix = format!("{}{}", prefix, sub_prefix);
                self.build_file_tree(&path, &new_prefix, output).await?;
            }
        }
        
        Ok(())
    }

    async fn extract_source_files(&self, repo_path: &Path, output_dir: &Path) -> Result<()> {
        let source_dir = output_dir.join("source");
        tokio_fs::create_dir_all(&source_dir).await?;
        
        let allowed_extensions = [
            "rs", "py", "js", "ts", "go", "c", "cpp", "h", "hpp",
            "java", "kt", "scala", "rb", "php", "sh", "bash", "md", "txt",
            "json", "yaml", "yml", "toml", "html", "css", "scss", "sql",
        ];
        
        self.process_directory(repo_path, &source_dir, &allowed_extensions).await?;
        
        Ok(())
    }

    async fn process_directory(&self, dir: &Path, output_base: &Path, allowed_extensions: &[&str]) -> Result<()> {
        let mut entries = Vec::new();
        let mut readdir = tokio_fs::read_dir(dir).await?;
        
        while let Some(entry) = readdir.next_entry().await? {
            entries.push(entry);
        }
            
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            
            // Skip hidden files and directories
            if path.file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.starts_with('.'))
                .unwrap_or(false) 
            {
                continue;
            }
            
            if file_type.is_dir() {
                self.process_directory(&path, output_base, allowed_extensions).await?;
            } else if file_type.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if allowed_extensions.contains(&ext) {
                        self.copy_source_file(&path, output_base).await?;
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn copy_source_file(&self, file_path: &Path, output_base: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ProcessorError::IO(e))?;
            
        let rel_path = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        let output_path = output_base.join(rel_path);
        
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        fs::write(&output_path, content)?;
        
        Ok(())
    }

    async fn save_binary_file(&self, bytes: &[u8], output_path: &Path) -> Result<()> {
        tokio_fs::write(output_path, bytes).await?;
        Ok(())
    }

    async fn save_text_file(&self, content: &str, output_path: &Path) -> Result<()> {
        let file_name = output_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();
            
        let outpath = output_path;
        
        if let Some(parent) = outpath.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let _outfile = std_fs::File::create(&outpath)?;
        std_fs::write(&outpath, content)?;
        
        Ok(())
    }

    async fn extract_zip_contents(&self, zip_path: &Path, extract_dir: &Path) -> Result<()> {
        // Create the extraction directory
        tokio_fs::create_dir_all(extract_dir).await?;
        
        // Open the zip file
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        // Extract each file
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_dir.join(file.name());
            
            if file.name().ends_with('/') {
                // It's a directory
                std::fs::create_dir_all(&outpath)?;
            } else {
                // It's a file
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        
        Ok(())
    }

    async fn process_repo_tree(&self, extract_path: &Path, output_dir: &Path, _pb: ProgressBar) -> Result<()> {
        let config = Config::load()?;
        
        // 1. Find root directory (usually there's only one directory in the extract path)
        let mut entries = Vec::new();
        let mut readdir = tokio_fs::read_dir(extract_path).await?;
        
        while let Some(entry) = readdir.next_entry().await? {
            entries.push(entry);
        }
        
        if entries.is_empty() {
            return Err(ProcessorError::Processing("Empty repository".to_string()));
        }
        
        let root_dir_entry = &entries[0];
        if !root_dir_entry.file_type().await?.is_dir() {
            return Err(ProcessorError::Processing("Invalid repository structure".to_string()));
        }
        
        let root_path = root_dir_entry.path();
        
        // 2. Process each file
        self.process_directory_recursive(&root_path, output_dir).await?;
        
        Ok(())
    }

    async fn process_directory_recursive(&self, dir: &Path, output_dir: &Path) -> Result<()> {
        let mut entries = Vec::new();
        let mut readdir = tokio_fs::read_dir(dir).await?;
        
        while let Some(entry) = readdir.next_entry().await? {
            entries.push(entry);
        }
        
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            
            // Skip .git and other dot directories
            if path.file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            
            if file_type.is_dir() {
                self.process_directory_recursive(&path, output_dir).await?;
            } else if file_type.is_file() {
                // Process the file based on its extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "rs" | "py" | "js" | "ts" | "go" | "java" => {
                            // Handle source code files
                            self.process_source_file(&path, output_dir).await?;
                        }
                        "md" | "txt" => {
                            // Handle text files
                            self.process_text_file(&path, output_dir).await?;
                        }
                        _ => {} // Skip other file types
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn process_source_file(&self, file_path: &Path, output_dir: &Path) -> Result<()> {
        let content = tokio_fs::read_to_string(file_path).await
            .map_err(|e| ProcessorError::IO(e))?;
            
        let file_name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        let rel_path = file_path.strip_prefix(output_dir.parent().unwrap_or(Path::new("")))
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
            
        let output_path = output_dir.join("source").join(file_name);
        
        if let Some(parent) = output_path.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }
        
        tokio_fs::write(&output_path, content).await?;
        
        Ok(())
    }

    async fn process_text_file(&self, file_path: &Path, output_dir: &Path) -> Result<()> {
        let content = tokio_fs::read_to_string(file_path).await
            .map_err(|e| ProcessorError::IO(e))?;
            
        let file_name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        let output_path = output_dir.join("docs").join(file_name);
        
        if let Some(parent) = output_path.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }
        
        tokio_fs::write(&output_path, content).await?;
        
        Ok(())
    }
}

#[async_trait]
impl PackageProcessor for GitHubProcessor {
    async fn process(&self, url: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        // Parse GitHub URL
        let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
        
        if parts.len() < 5 || parts[2] != "github.com" {
            return Err(ProcessorError::InvalidUrl(
                "Invalid GitHub URL format. Expected: https://github.com/owner/repo".to_string()
            ));
        }
        
        let owner = parts[3];
        let repo = parts[4];
        
        // Process the repository
        self.download_repo(owner, repo, output_dir).await?;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "github"
    }
    
    fn accepts(&self, url: &str) -> bool {
        url.contains("github.com/")
    }
}
EOL

echo "✅ GitHub processor fixed!"
