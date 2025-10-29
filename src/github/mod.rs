use crate::error::{ProcessorError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        Ok(Self { client, token })
    }

    pub async fn check_rate_limit(&self) -> Result<()> {
        let response = self.client
            .get("https://api.github.com/rate_limit")
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProcessorError::RateLimitExceeded(
                response.status().to_string()
            ));
        }

        Ok(())
    }

    pub async fn get_repository(&self, repo: &str) -> Result<Repository> {
        let url = format!("https://api.github.com/repos/{}", repo);
        
        let mut request = self.client.get(&url);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProcessorError::GitHubApi(format!(
                "Failed to fetch repository: {}", response.status()
            )));
        }

        response.json::<Repository>()
            .await
            .map_err(|e| ProcessorError::Processing(e.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub default_branch: Option<String>,
} 