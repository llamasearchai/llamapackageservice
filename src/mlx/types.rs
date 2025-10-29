use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub language_stats: HashMap<String, usize>,
    pub key_dependencies: Vec<String>,
    pub complexity_metrics: ComplexityMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: f32,
    pub cognitive_complexity: f32,
    pub maintainability_index: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MLXResponse {
    pub embeddings: Option<Vec<f32>>,
    pub analysis: Option<CodeAnalysis>,
    pub insights: Option<SemanticInsights>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticInsights {
    pub key_concepts: Vec<String>,
    pub architecture_patterns: Vec<String>,
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
} 