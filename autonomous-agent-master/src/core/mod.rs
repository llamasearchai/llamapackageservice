/// Core decision engine and autonomous intelligence
pub mod decision_engine;
pub mod context;
pub mod decisions;
pub mod rules;
pub mod strategies;

pub use decision_engine::{DecisionEngine, DecisionConfig};
pub use context::Context;
pub use decisions::{Decision, Priority};
pub use rules::{Rule, RuleEngine};
pub use strategies::{Strategy, StrategyType};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Resource requirements for scaling decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<u32>,
    pub memory_gb: Option<u32>,
    pub storage_gb: Option<u32>,
    pub gpu_count: Option<u32>,
    pub network_bandwidth_mbps: Option<u32>,
}

/// Anomaly detected in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub affected_components: Vec<String>,
    pub metrics: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    Performance,
    Security,
    Reliability,
    Cost,
    Compliance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}