//! OpenAI Agents SDK Integration Module
//!
//! This module provides integration with OpenAI's API for intelligent
//! code analysis, repository understanding, and automated documentation generation.
//!
//! Author: Nik Jois <nikjois@llamasearch.ai>

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::{ProcessorError, Result};

pub mod analysis;
pub mod conversation;
pub mod tools;

/// OpenAI Agents client wrapper
#[derive(Clone)]
pub struct OpenAIAgent {
    api_key: String,
    config: AgentConfig,
}

/// Configuration for OpenAI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// OpenAI API key
    pub api_key: String,
    /// Model to use for analysis
    pub model: String,
    /// Maximum tokens per request
    pub max_tokens: u16,
    /// Temperature for response generation
    pub temperature: f32,
    /// System prompt for repository analysis
    pub system_prompt: String,
}

/// Repository analysis request
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRequest {
    /// Repository URL or path
    pub repository: String,
    /// Type of analysis to perform
    pub analysis_type: AnalysisType,
    /// Additional context or instructions
    pub context: Option<String>,
    /// Custom parameters for the analysis
    pub parameters: HashMap<String, String>,
}

/// Types of analysis that can be performed
#[derive(Debug, Serialize, Deserialize)]
pub enum AnalysisType {
    /// Generate comprehensive repository documentation
    Documentation,
    /// Analyze code quality and suggest improvements
    CodeReview,
    /// Generate API documentation
    ApiDocumentation,
    /// Analyze dependencies and security
    SecurityAudit,
    /// Generate usage examples
    Examples,
    /// Custom analysis with user-defined prompt
    Custom(String),
}

/// Analysis result from OpenAI agent
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Unique identifier for the analysis
    pub id: String,
    /// Type of analysis performed
    pub analysis_type: AnalysisType,
    /// Generated content
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Metadata about the analysis
    pub metadata: HashMap<String, String>,
    /// Timestamp of analysis
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Conversation context for interactive analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Conversation ID
    pub id: String,
    /// Message history
    pub messages: Vec<Message>,
    /// Repository context
    pub repository_context: String,
    /// Analysis session state
    pub session_state: HashMap<String, String>,
}

/// Individual message in a conversation
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: String,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl OpenAIAgent {
    /// Create a new OpenAI agent
    pub fn new(config: AgentConfig) -> Result<Self> {
        Ok(Self {
            api_key: config.api_key.clone(),
            config,
        })
    }

    /// Create agent from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProcessorError::new("OPENAI_API_KEY environment variable not set"))?;
        
        let config = AgentConfig {
            api_key: api_key.clone(),
            model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            max_tokens: std::env::var("OPENAI_MAX_TOKENS")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .unwrap_or(4000),
            temperature: std::env::var("OPENAI_TEMPERATURE")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
            system_prompt: include_str!("../../prompts/system_prompt.txt").to_string(),
        };
        
        Self::new(config)
    }

    /// Analyze a repository using OpenAI
    pub async fn analyze_repository(&self, request: AnalysisRequest) -> Result<AnalysisResult> {
        // For now, return a mock result until we fix the OpenAI integration
        let result = AnalysisResult {
            id: uuid::Uuid::new_v4().to_string(),
            analysis_type: request.analysis_type,
            content: format!("Mock analysis for repository: {}", request.repository),
            confidence: 0.8,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        Ok(result)
    }

    /// Start an interactive conversation about a repository
    pub async fn start_conversation(&self, repository: String) -> Result<ConversationContext> {
        let id = uuid::Uuid::new_v4().to_string();
        let repository_context = self.load_repository_context(&repository).await?;
        
        let context = ConversationContext {
            id,
            messages: Vec::new(),
            repository_context,
            session_state: HashMap::new(),
        };

        Ok(context)
    }

    /// Continue a conversation with user input
    pub async fn continue_conversation(
        &self,
        context: &mut ConversationContext,
        user_message: String,
    ) -> Result<String> {
        // Add user message to context
        context.messages.push(Message {
            role: "user".to_string(),
            content: user_message.clone(),
            timestamp: chrono::Utc::now(),
        });

        // For now, return a mock response until we fix the OpenAI integration
        let response = format!("Mock response to: {}", user_message);

        // Add assistant message to context
        context.messages.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
            timestamp: chrono::Utc::now(),
        });

        Ok(response)
    }

    /// Generate code examples for a repository
    pub async fn generate_examples(&self, repository: String) -> Result<Vec<String>> {
        let request = AnalysisRequest {
            repository: repository.clone(),
            analysis_type: AnalysisType::Examples,
            context: Some("Generate practical usage examples".to_string()),
            parameters: HashMap::new(),
        };

        let result = self.analyze_repository(request).await?;
        
        // Parse examples from the response
        let examples: Vec<String> = result.content
            .split("```")
            .enumerate()
            .filter_map(|(i, section)| {
                if i % 2 == 1 { // Odd indices are code blocks
                    Some(section.to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(examples)
    }

    /// Perform security audit using AI
    pub async fn security_audit(&self, repository: String) -> Result<SecurityAuditResult> {
        let request = AnalysisRequest {
            repository: repository.clone(),
            analysis_type: AnalysisType::SecurityAudit,
            context: Some("Analyze security vulnerabilities and best practices".to_string()),
            parameters: HashMap::new(),
        };

        let result = self.analyze_repository(request).await?;
        
        // Parse security audit results
        let audit_result = SecurityAuditResult {
            repository,
            vulnerabilities: self.parse_vulnerabilities(&result.content),
            recommendations: self.parse_recommendations(&result.content),
            risk_score: self.calculate_risk_score(&result.content),
            timestamp: result.timestamp,
        };

        Ok(audit_result)
    }

    // Helper methods
    fn build_system_prompt(&self, request: &AnalysisRequest) -> String {
        let base_prompt = &self.config.system_prompt;
        
        let analysis_specific = match &request.analysis_type {
            AnalysisType::Documentation => {
                "Focus on creating comprehensive, well-structured documentation."
            },
            AnalysisType::CodeReview => {
                "Analyze code quality, patterns, and suggest improvements."
            },
            AnalysisType::ApiDocumentation => {
                "Generate detailed API documentation with examples."
            },
            AnalysisType::SecurityAudit => {
                "Identify security vulnerabilities and best practices."
            },
            AnalysisType::Examples => {
                "Create practical, working examples of how to use this code."
            },
            AnalysisType::Custom(prompt) => prompt,
        };

        format!("{}\n\nSpecific Instructions: {}", base_prompt, analysis_specific)
    }

    async fn load_repository_context(&self, repository: &str) -> Result<String> {
        // This would load and summarize repository content
        // For now, return a placeholder
        Ok(format!("Repository: {}\nLoaded basic context", repository))
    }

    fn parse_vulnerabilities(&self, content: &str) -> Vec<String> {
        // Parse vulnerability information from AI response
        content.lines()
            .filter(|line| line.to_lowercase().contains("vulnerability") || 
                          line.to_lowercase().contains("security"))
            .map(|line| line.to_string())
            .collect()
    }

    fn parse_recommendations(&self, content: &str) -> Vec<String> {
        // Parse recommendations from AI response
        content.lines()
            .filter(|line| line.to_lowercase().contains("recommend") || 
                          line.to_lowercase().contains("should"))
            .map(|line| line.to_string())
            .collect()
    }

    fn calculate_risk_score(&self, content: &str) -> u8 {
        // Simple risk calculation based on content
        let risk_keywords = ["critical", "high", "vulnerability", "exploit"];
        let risk_count = risk_keywords.iter()
            .map(|keyword| content.to_lowercase().matches(keyword).count())
            .sum::<usize>();
        
        std::cmp::min(risk_count as u8 * 20, 100)
    }
}

/// Security audit result
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityAuditResult {
    /// Repository that was audited
    pub repository: String,
    /// Identified vulnerabilities
    pub vulnerabilities: Vec<String>,
    /// Security recommendations
    pub recommendations: Vec<String>,
    /// Risk score (0-100)
    pub risk_score: u8,
    /// Timestamp of audit
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gpt-4".to_string(),
            max_tokens: 4000,
            temperature: 0.7,
            system_prompt: "You are an expert software engineer and code analyst.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 4000);
    }

    #[test]
    fn test_analysis_request_serialization() {
        let request = AnalysisRequest {
            repository: "https://github.com/test/repo".to_string(),
            analysis_type: AnalysisType::Documentation,
            context: None,
            parameters: HashMap::new(),
        };
        
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: AnalysisRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.repository, request.repository);
    }
} 