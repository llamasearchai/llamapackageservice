use crate::error::ProcessorError;
use crate::processors::common::{self, setup_progress_style};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;
use crate::processors::github::generate_repo_index;
use chrono::Utc;

pub async fn process_github_org(
    org_url: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<(), ProcessorError> {
    setup_progress_style(pb);
    pb.set_message("Processing GitHub organization...");

    // Extract organization name from URL
    let parts: Vec<_> = org_url.trim_end_matches('/').split('/').collect();
    let org_name = parts.last().unwrap_or(&"unknown").to_string();

    // Get organization profile info
    let org_info = get_org_info(&org_name).await?;

    // Create master index with organization info
    let mut master_index = String::new();
    master_index.push_str(&format!("# GitHub Organization: {}\n\n", org_name));
    master_index.push_str(&format!("Source: {}\n", org_url));
    
    // Add organization details
    if let Some(description) = org_info.get("description").and_then(|v| v.as_str()) {
        master_index.push_str(&format!("\n## Description\n{}\n", description));
    }
    if let Some(blog) = org_info.get("blog").and_then(|v| v.as_str()) {
        master_index.push_str(&format!("\nWebsite: {}\n", blog));
    }
    if let Some(location) = org_info.get("location").and_then(|v| v.as_str()) {
        master_index.push_str(&format!("\nLocation: {}\n", location));
    }
    master_index.push_str(&format!("\nPublic Repositories: {}\n", 
        org_info.get("public_repos").and_then(|v| v.as_u64()).unwrap_or(0)));
    
    master_index.push_str("\n## Repositories\n\n");

    // Get list of repositories
    let repos = get_org_repos(&org_name).await?;
    pb.set_length(repos.len() as u64);
    
    // Set up a more detailed progress style
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} repos processed | {msg}")
        .unwrap());

    // Process each repository
    for (i, repo) in repos.iter().enumerate() {
        pb.set_position(i as u64);
        
        let repo_name = repo["name"].as_str().unwrap_or("unknown");
        let repo_url = repo["html_url"].as_str().unwrap_or("");
        
        pb.set_message(format!("Processing {}", repo_name));
        
        // Add repository metadata
        master_index.push_str(&format!("### {}\n", repo_name));
        master_index.push_str(&format!("URL: {}\n", repo_url));
        
        if let Some(description) = repo["description"].as_str() {
            master_index.push_str(&format!("Description: {}\n", description));
        }
        if let Some(language) = repo["language"].as_str() {
            master_index.push_str(&format!("Primary Language: {}\n", language));
        }
        master_index.push_str(&format!("Stars: {}\n", 
            repo["stargazers_count"].as_u64().unwrap_or(0)));
        master_index.push_str(&format!("Forks: {}\n", 
            repo["forks_count"].as_u64().unwrap_or(0)));
        
        // Process repository contents
        let temp_dir = tempfile::tempdir().map_err(|e| ProcessorError::IO(e))?;
        match common::download_github_repo(repo_url, pb).await {
            Ok(archive_bytes) => {
                if let Ok(()) = common::extract_zip_archive(&archive_bytes, temp_dir.path(), Some(pb)).await {
                    if let Ok(repo_index) = generate_repo_index(temp_dir.path()) {
                        master_index.push_str("\nContents:\n");
                        master_index.push_str(&repo_index);
                    }
                }
            },
            Err(e) => {
                master_index.push_str(&format!("\nError processing repository: {}\n", e));
            }
        }
        master_index.push_str("\n---\n\n");
    }

    // Save the complete organization profile
    let timestamp = Utc::now().timestamp();
    let file_name = format!("{}_github_org_{}.txt", org_name, timestamp);
    let file_path = output_dir.join(file_name);
    
    fs::write(file_path, master_index).map_err(|e| ProcessorError::IO(e)).await?;
    
    pb.finish_with_message(format!("✨ GitHub organization {} processed successfully", org_name));
    Ok(())
}

async fn get_org_info(org_name: &str) -> Result<Value, ProcessorError> {
    let client = Client::new();
    let api_url = format!("https://api.github.com/orgs/{}", org_name);
    
    let response = client.get(&api_url)
        .header("User-Agent", "LlamaSearch")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    response.json()
        .await
        .map_err(|e| ProcessorError::Processing(e.to_string()))
}

async fn get_org_repos(org_name: &str) -> Result<Vec<Value>, ProcessorError> {
    let client = Client::new();
    let mut all_repos = Vec::new();
    let mut page = 1;
    
    loop {
        let api_url = format!(
            "https://api.github.com/orgs/{}/repos?page={}&per_page=100",
            org_name, page
        );
        
        let response = client.get(&api_url)
            .header("User-Agent", "LlamaSearch")
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
            
        let repos: Vec<Value> = response.json()
            .await
            .map_err(|e| ProcessorError::Processing(e.to_string()))?;
            
        if repos.is_empty() {
            break;
        }
        
        all_repos.extend(repos);
        page += 1;
    }
    
    Ok(all_repos)
} 