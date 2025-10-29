/// Autonomous agents for task execution
pub mod orchestrator;
pub mod specialized;
pub mod capabilities;
pub mod coordination;

pub use orchestrator::{AgentOrchestrator, AgentConfig};
pub use specialized::{SpecializedAgent, AgentType};
pub use capabilities::Capability;
pub use coordination::CoordinationProtocol;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A task to be executed by agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: crate::core::Priority,
    pub requirements: Vec<Capability>,
    pub dependencies: Vec<String>,
    pub context: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub assigned_agent: Option<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Analysis,
    Development,
    Testing,
    Deployment,
    Monitoring,
    Optimization,
    Documentation,
    Communication,
    Learning,
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: serde_json::Value,
    pub artifacts: Vec<Artifact>,
    pub metrics: TaskMetrics,
    pub errors: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub artifact_type: String,
    pub location: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub execution_time: std::time::Duration,
    pub resource_usage: ResourceUsage,
    pub quality_score: f64,
    pub complexity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_seconds: f64,
    pub memory_mb: f64,
    pub network_bytes: u64,
    pub disk_bytes: u64,
}