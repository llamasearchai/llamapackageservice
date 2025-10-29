use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCallResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub trait LLMFunction {
    fn get_definition() -> FunctionDefinition;
    async fn execute(args: serde_json::Value) -> FunctionCallResponse;
}

// Example implementation for GitHub processing
#[cfg(feature = "llm")]
pub mod github {
    use super::*;
    use crate::processors::github::llm::ProcessGitHubArgs;

    pub struct ProcessGitHubFunction;

    impl LLMFunction for ProcessGitHubFunction {
        fn get_definition() -> FunctionDefinition {
            FunctionDefinition {
                name: "process_github".to_string(),
                description: "Process a GitHub repository or organization".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "GitHub repository or organization URL"
                        },
                        "output_path": {
                            "type": "string",
                            "description": "Path where to save the output"
                        }
                    },
                    "required": ["url", "output_path"]
                }),
            }
        }

        async fn execute(args: serde_json::Value) -> FunctionCallResponse {
            let args: ProcessGitHubArgs = match serde_json::from_value(args) {
                Ok(a) => a,
                Err(e) => return FunctionCallResponse {
                    success: false,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                },
            };

            match crate::processors::github::llm::process_github(args).await {
                Ok(msg) => FunctionCallResponse {
                    success: true,
                    message: msg,
                    data: None,
                },
                Err(e) => FunctionCallResponse {
                    success: false,
                    message: e,
                    data: None,
                },
            }
        }
    }
} 