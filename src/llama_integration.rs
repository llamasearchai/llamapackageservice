use crate::error::{ProcessorError, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

pub struct LlamaClient {
    client: Client,
    endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaRequest {
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaResponse {
    pub text: String,
}

impl LlamaClient {
    pub fn new(endpoint: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self { client, endpoint }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let request = LlamaRequest {
            prompt: prompt.to_string(),
            max_tokens: 1000,
            temperature: 0.7,
        };

        let response = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProcessorError::LLM(e.to_string()))?;

        let llama_response: LlamaResponse = response
            .json()
            .await
            .map_err(|e| ProcessorError::LLM(e.to_string()))?;

        Ok(llama_response.text)
    }
}
