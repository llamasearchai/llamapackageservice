use crate::error::ProcessorError;
use crate::config::Config;
use crate::mlx::types::*;
use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};

const MLX_API_TIMEOUT: u64 = 30;
const MLX_ENDPOINT: &str = "http://localhost:5001/v1";

pub struct MLXClient {
    client: Client,
    config: Config,
    base_url: String,
}

impl MLXClient {
    pub fn new(config: &Config) -> Result<Self, ProcessorError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(MLX_API_TIMEOUT))
            .build()
            .map_err(|e| ProcessorError::Network(e.to_string()))?;

        Ok(Self {
            client,
            config: config.clone(),
            base_url: MLX_ENDPOINT.to_string(),
        })
    }

    pub async fn analyze_repository(&self, content: &str) -> Result<MLXResponse, ProcessorError> {
        info!("Starting MLX analysis");
        
        // Generate embeddings first
        let embeddings = self.generate_embeddings(content).await?;
        
        // Run code analysis
        let analysis = self.analyze_code(content, &embeddings).await?;
        
        // Generate insights
        let insights = self.generate_insights(content).await?;

        Ok(MLXResponse {
            embeddings: Some(embeddings),
            analysis: Some(analysis),
            insights: Some(insights),
        })
    }

    pub async fn generate_embeddings(&self, content: &str) -> Result<Vec<f32>, ProcessorError> {
        let response = self.client.post("http://localhost:5001/v1/embeddings")
            .json(&serde_json::json!({
                "model": "code-embedding-mlx",
                "input": content
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.embeddings)
    }

    pub async fn analyze_code(&self, content: &str, embeddings: &[f32]) -> Result<CodeAnalysis, ProcessorError> {
        let response = self.client.post("http://localhost:5001/v1/completions")
            .json(&serde_json::json!({
                "model": "code-llama-mlx",
                "messages": [{
                    "role": "system",
                    "content": "Analyze code repository content and provide structured insights."
                }, {
                    "role": "user", 
                    "content": content
                }],
                "embeddings": embeddings,
                "functions": [{
                    "name": "analyze_code",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "language_stats": {
                                "type": "object",
                                "additionalProperties": {"type": "integer"}
                            },
                            "key_dependencies": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "complexity_metrics": {
                                "type": "object",
                                "properties": {
                                    "cyclomatic_complexity": {"type": "number"},
                                    "cognitive_complexity": {"type": "number"},
                                    "maintainability_index": {"type": "number"}
                                }
                            }
                        }
                    }
                }]
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    pub async fn generate_insights(&self, content: &str) -> Result<SemanticInsights, ProcessorError> {
        let response = self.client.post("http://localhost:5001/v1/chat/completions")
            .json(&serde_json::json!({
                "model": "code-llama-mlx",
                "messages": [{
                    "role": "system",
                    "content": "Generate semantic insights about the codebase."
                }, {
                    "role": "user",
                    "content": content
                }],
                "temperature": 0.2
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_mlx_client() {
        let mut server = Server::new();
        
        let mock = server.mock("POST", "/v1/completions")
            .with_status(200)
            .with_body(r#"{"analysis": {"complexity": 0.75}}"#)
            .create();

        let config = Config::default();
        let client = MLXClient::new(&config).unwrap();
        
        let result = client.analyze_code("test code", &[0.1, 0.2]).await;
        assert!(result.is_ok());
        
        mock.assert();
    }
} 