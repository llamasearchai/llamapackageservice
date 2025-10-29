use crate::error::{ProcessorError, Result};
use crate::config::Config;
use super::types::*;
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};
use serde_json::json;

const MLX_API_TIMEOUT: u64 = 30;

pub struct MLXClient {
    client: Client,
    config: Config,
    base_url: String,
}

impl MLXClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.mlx_settings.timeout_seconds))
            .build()
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        Ok(Self {
            client,
            config: config.clone(),
            base_url: config.mlx_settings.endpoint.clone(),
        })
    }

    pub async fn analyze_repository(&self, content: &str) -> Result<MLXResponse> {
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

    pub async fn generate_embeddings(&self, content: &str) -> Result<Vec<f32>> {
        let response = self.client.post(&format!("{}/embeddings", self.base_url))
            .json(&json!({
                "model": "code-embedding-mlx",
                "input": content
            }))
            .send()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        let embeddings = response["data"][0]["embedding"].as_array()
            .ok_or_else(|| ProcessorError::MLX("Invalid embedding response format".to_string()))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        Ok(embeddings)
    }

    pub async fn analyze_code(&self, content: &str, embeddings: &[f32]) -> Result<CodeAnalysis> {
        let response = self.client.post(&format!("{}/completions", self.base_url))
            .json(&json!({
                "model": self.config.mlx_settings.model_name,
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
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProcessorError::MLX(format!(
                "Failed to analyze code: HTTP {}", response.status()
            )));
        }

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        let function_call = result["choices"][0]["message"]["function_call"].as_object()
            .ok_or_else(|| ProcessorError::MLX("Invalid function call response".to_string()))?;

        let arguments = function_call["arguments"].as_str()
            .ok_or_else(|| ProcessorError::MLX("Missing function arguments".to_string()))?;

        let analysis: CodeAnalysis = serde_json::from_str(arguments)
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        Ok(analysis)
    }

    pub async fn generate_insights(&self, content: &str) -> Result<SemanticInsights> {
        let response = self.client.post(&format!("{}/chat/completions", self.base_url))
            .json(&json!({
                "model": self.config.mlx_settings.model_name,
                "messages": [{
                    "role": "system",
                    "content": "Generate semantic insights about the codebase. Identify key concepts, architecture patterns, and improvement suggestions."
                }, {
                    "role": "user",
                    "content": content
                }],
                "temperature": 0.2,
                "function_call": {
                    "name": "generate_insights"
                },
                "functions": [{
                    "name": "generate_insights",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "key_concepts": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "architecture_patterns": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "improvement_suggestions": {
                                "type": "array",
                                "items": {"type": "string"}
                            }
                        }
                    }
                }]
            }))
            .send()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProcessorError::MLX(format!(
                "Failed to generate insights: HTTP {}", response.status()
            )));
        }

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        let function_call = result["choices"][0]["message"]["function_call"].as_object()
            .ok_or_else(|| ProcessorError::MLX("Invalid function call response".to_string()))?;

        let arguments = function_call["arguments"].as_str()
            .ok_or_else(|| ProcessorError::MLX("Missing function arguments".to_string()))?;

        let insights: SemanticInsights = serde_json::from_str(arguments)
            .map_err(|e| ProcessorError::Message(e.to_string()))?;

        Ok(insights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_mlx_client() {
        let mut server = Server::new();
        
        // Mock the embeddings endpoint
        let mock_embeddings = server.mock("POST", "/embeddings")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data": [{"embedding": [0.1, 0.2, 0.3, 0.4, 0.5]}]}"#)
            .create();
            
        // Mock the completions endpoint
        let mock_completions = server.mock("POST", "/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices": [{"message": {"function_call": {"name": "analyze_code", "arguments": "{\"language_stats\": {\"Rust\": 80, \"Python\": 20}, \"key_dependencies\": [\"serde\", \"tokio\"], \"complexity_metrics\": {\"cyclomatic_complexity\": 12.5, \"cognitive_complexity\": 8.3, \"maintainability_index\": 85.2}}"}}}]}"#)
            .create();
            
        // Mock the chat completions endpoint
        let mock_insights = server.mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices": [{"message": {"function_call": {"name": "generate_insights", "arguments": "{\"key_concepts\": [\"API\", \"Processing\", \"Analysis\"], \"architecture_patterns\": [\"Microservices\", \"Event-driven\"], \"improvement_suggestions\": [\"Add more tests\", \"Improve documentation\"]}"}}}]}"#)
            .create();
            
        // Create config pointing to our mock server
        let mut config = Config::default();
        config.mlx_settings.endpoint = format!("{}", server.url());
        
        let client = MLXClient::new(&config).unwrap();
        
        // Test the analyze_repository method
        let result = client.analyze_repository("test code").await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.embeddings.is_some());
        assert!(response.analysis.is_some());
        assert!(response.insights.is_some());
        
        // Verify the analysis data
        if let Some(analysis) = response.analysis {
            assert_eq!(analysis.language_stats.get("Rust"), Some(&80));
            assert_eq!(analysis.key_dependencies.len(), 2);
            assert!(analysis.complexity_metrics.cyclomatic_complexity > 0.0);
        }
        
        // Verify insights data
        if let Some(insights) = response.insights {
            assert_eq!(insights.key_concepts.len(), 3);
            assert_eq!(insights.architecture_patterns.len(), 2);
            assert_eq!(insights.improvement_suggestions.len(), 2);
        }
        
        mock_embeddings.assert();
        mock_completions.assert();
        mock_insights.assert();
    }
}
