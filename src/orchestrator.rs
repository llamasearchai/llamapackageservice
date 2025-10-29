use crate::error::ProcessorError;
use crate::config::Config;
use crate::processors::github::GitHubProcessor;
use crate::rate_limiter::RateLimiter;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub repository_analysis: RepositoryAnalysis,
    pub code_metrics: CodeMetrics,
    pub semantic_insights: SemanticInsights,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryAnalysis {
    pub language_breakdown: std::collections::HashMap<String, f32>,
    pub total_files: usize,
    pub total_lines: usize,
    pub complexity_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub cyclomatic_complexity: f32,
    pub cognitive_complexity: f32,
    pub maintainability_index: f32,
    pub dependency_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticInsights {
    pub key_concepts: Vec<String>,
    pub architecture_patterns: Vec<String>,
    pub improvement_suggestions: Vec<String>,
}

pub struct ProcessingOrchestrator {
    config: Config,
    github_processor: Arc<GitHubProcessor>,
    rate_limiter: Arc<RateLimiter>,
    mlx_client: Arc<MLXClient>,
}

impl ProcessingOrchestrator {
    pub fn new(config: Config) -> Result<Self, ProcessorError> {
        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limits.github_requests_per_hour,
            "GitHub"
        ));

        let github_processor = Arc::new(GitHubProcessor::new(
            &config.output.base_dir,
            config.clone(),
        )?);

        let mlx_client = Arc::new(MLXClient::new(&config)?);

        Ok(Self {
            config,
            github_processor,
            rate_limiter,
            mlx_client,
        })
    }

    pub async fn process_repository(&self, url: &str) -> Result<ProcessingResult, ProcessorError> {
        info!("Starting repository processing for {}", url);
        
        // Process GitHub repository
        let repo_content = self.github_processor.process_url(url).await?;
        
        // Generate embeddings
        let embeddings = self.mlx_client.generate_embeddings(&repo_content).await?;
        
        // Analyze code with MLX
        let code_analysis = self.mlx_client.analyze_code(&repo_content, &embeddings).await?;
        
        // Generate semantic insights
        let semantic_insights = self.mlx_client.generate_insights(&repo_content).await?;

        Ok(ProcessingResult {
            repository_analysis: code_analysis.into(),
            code_metrics: code_analysis.metrics,
            semantic_insights,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use tokio;

    #[tokio::test]
    async fn test_full_processing_pipeline() {
        let mut server = Server::new();
        
        // Mock GitHub API
        let github_mock = server.mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../test_data/github_response.json"))
            .create();

        // Mock MLX API
        let mlx_mock = server.mock("POST", "/v1/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../test_data/mlx_response.json"))
            .create();

        let config = Config::default();
        let orchestrator = ProcessingOrchestrator::new(config).unwrap();

        let result = orchestrator.process_repository("https://github.com/owner/repo").await;
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert!(analysis.code_metrics.maintainability_index > 0.0);
        assert!(!analysis.semantic_insights.key_concepts.is_empty());

        github_mock.assert();
        mlx_mock.assert();
    }
} 