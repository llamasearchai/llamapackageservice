use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use colored::*;
use tracing::info;
use crate::error::ProcessorError;

#[derive(Debug, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub download_url: String,
}

impl CrateInfo {
    pub async fn from_url(url: &str) -> Result<Self, ProcessorError> {
        let client = reqwest::Client::new();
        
        info!("Extracting crate info from URL: {}", url);
        
        let crate_name = url.split('/')
            .last()
            .unwrap_or(url)
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
        
        let api_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        info!("Fetching crate info from: {}", api_url);
        println!("Fetching crate info from: {}", api_url.cyan());
        
        let response = client.get(&api_url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
            
        if !response.status().is_success() {
            return Err(ProcessorError::Network(
                format!("Failed to fetch crate info: HTTP {}", response.status())
            ));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| ProcessorError::Processing(e.to_string()))?;
            
        let crate_data = data["crate"].as_object()
            .ok_or_else(|| ProcessorError::Processing("Invalid crates.io response".to_string()))?;
        
        let version = crate_data["max_version"]
            .as_str()
            .unwrap_or("0.1.0")
            .to_string();
            
        let download_url = format!(
            "https://static.crates.io/crates/{crate_name}/{crate_name}-{version}.crate",
            crate_name = crate_name,
            version = version
        );

        Ok(CrateInfo {
            name: crate_name.to_string(),
            version,
            description: crate_data["description"].as_str().map(String::from),
            documentation: crate_data["documentation"].as_str().map(String::from),
            repository: crate_data["repository"].as_str().map(String::from),
            download_url,
        })
    }

    pub async fn fetch_docs(&self) -> Result<String, ProcessorError> {
        let client = super::pypi::create_stealth_client()?;
        
        info!("Attempting to fetch documentation for {}", self.name);
        
        // Try docs.rs first
        let docs_url = format!("https://docs.rs/{}/{}", self.name, self.version);
        let response = client.get(&docs_url)
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;
            
        if response.status().is_success() {
            Ok(response.text().await
                .map_err(|e| ProcessorError::Network(e.to_string()))?)
        } else if let Some(doc_url) = &self.documentation {
            info!("Falling back to crate documentation URL");
            let response = client.get(doc_url)
                .send()
                .await
                .map_err(|e| ProcessorError::Network(e.to_string()))?;
            Ok(response.text().await
                .map_err(|e| ProcessorError::Network(e.to_string()))?)
        } else {
            Err(ProcessorError::Processing("No documentation available".to_string()))
        }
    }
}

pub async fn download_crate(crate_info: &CrateInfo, output_path: &str) -> Result<(), ProcessorError> {
    let client = reqwest::Client::new();
    
    info!("Downloading crate from: {}", crate_info.download_url);
    println!("Downloading crate from: {}", crate_info.download_url.cyan());
    
    let response = client.get(&crate_info.download_url)
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    if !response.status().is_success() {
        return Err(ProcessorError::Network(
            format!("Failed to download crate: HTTP {}", response.status())
        ));
    }
    
    let bytes = response.bytes().await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
        
    let mut file = File::create(output_path)
        .map_err(|e| ProcessorError::IO(e))?;
        
    file.write_all(&bytes)
        .map_err(|e| ProcessorError::IO(e))?;
    
    Ok(())
}

pub fn extract_crate(crate_path: &str, extract_path: &str) -> Result<(), ProcessorError> {
    use std::process::Command;
    
    info!("Extracting crate from {} to {}", crate_path, extract_path);
    
    std::fs::create_dir_all(extract_path)
        .map_err(|e| ProcessorError::IO(e))?;
    
    let output = Command::new("tar")
        .args(&["xf", crate_path, "-C", extract_path])
        .output()
        .map_err(|e| ProcessorError::Processing(format!("Failed to extract crate: {}", e)))?;
        
    if !output.status.success() {
        return Err(ProcessorError::Processing(
            format!("Failed to extract crate: {}", 
                String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
} 