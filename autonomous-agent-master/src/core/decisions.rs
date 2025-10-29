/// Decision types and structures
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::{
    agents::Task,
    execution::Workflow,
    core::ResourceRequirements,
};

/// Autonomous decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    /// Execute a specific task
    ExecuteTask(Task),
    
    /// Trigger a workflow
    TriggerWorkflow(Workflow),
    
    /// Update system configuration
    UpdateConfiguration(serde_json::Value),
    
    /// Scale resources up or down
    ScaleResources(ResourceRequirements),
    
    /// Investigate an anomaly
    InvestigateAnomaly(super::Anomaly),
    
    /// Deploy a new version
    Deploy(DeploymentDecision),
    
    /// Optimize a component
    Optimize(OptimizationDecision),
    
    /// Heal a failed component
    Heal(HealingDecision),
    
    /// Learn from data
    Learn(LearningDecision),
    
    /// Communicate with external system
    Communicate(CommunicationDecision),
}

impl Decision {
    /// Get unique ID for the decision
    pub fn id(&self) -> String {
        match self {
            Decision::ExecuteTask(task) => task.id.clone(),
            Decision::TriggerWorkflow(workflow) => workflow.id.clone(),
            Decision::UpdateConfiguration(_) => uuid::Uuid::new_v4().to_string(),
            Decision::ScaleResources(_) => uuid::Uuid::new_v4().to_string(),
            Decision::InvestigateAnomaly(anomaly) => anomaly.id.clone(),
            Decision::Deploy(d) => d.id.clone(),
            Decision::Optimize(o) => o.id.clone(),
            Decision::Heal(h) => h.id.clone(),
            Decision::Learn(l) => l.id.clone(),
            Decision::Communicate(c) => c.id.clone(),
        }
    }
    
    /// Get priority of the decision
    pub fn priority(&self) -> Priority {
        match self {
            Decision::ExecuteTask(task) => task.priority.clone(),
            Decision::TriggerWorkflow(_) => Priority::Normal,
            Decision::UpdateConfiguration(_) => Priority::High,
            Decision::ScaleResources(_) => Priority::High,
            Decision::InvestigateAnomaly(anomaly) => {
                match anomaly.severity {
                    super::Severity::Critical => Priority::Critical,
                    super::Severity::High => Priority::High,
                    super::Severity::Medium => Priority::Normal,
                    super::Severity::Low => Priority::Low,
                }
            },
            Decision::Deploy(d) => d.priority.clone(),
            Decision::Optimize(_) => Priority::Low,
            Decision::Heal(h) => h.priority.clone(),
            Decision::Learn(_) => Priority::Low,
            Decision::Communicate(c) => c.priority.clone(),
        }
    }
    
    /// Check if decision is urgent
    pub fn is_urgent(&self) -> bool {
        matches!(self.priority(), Priority::Critical | Priority::High)
    }
    
    /// Estimate the impact of the decision (0.0 - 1.0)
    pub fn estimated_impact(&self) -> f64 {
        match self {
            Decision::ExecuteTask(_) => 0.3,
            Decision::TriggerWorkflow(_) => 0.5,
            Decision::UpdateConfiguration(_) => 0.7,
            Decision::ScaleResources(_) => 0.8,
            Decision::InvestigateAnomaly(a) => {
                match a.severity {
                    super::Severity::Critical => 1.0,
                    super::Severity::High => 0.8,
                    super::Severity::Medium => 0.5,
                    super::Severity::Low => 0.3,
                }
            },
            Decision::Deploy(_) => 0.9,
            Decision::Optimize(_) => 0.4,
            Decision::Heal(_) => 0.8,
            Decision::Learn(_) => 0.2,
            Decision::Communicate(_) => 0.3,
        }
    }
    
    /// Get decision type as string
    pub fn decision_type(&self) -> &'static str {
        match self {
            Decision::ExecuteTask(_) => "execute_task",
            Decision::TriggerWorkflow(_) => "trigger_workflow",
            Decision::UpdateConfiguration(_) => "update_configuration",
            Decision::ScaleResources(_) => "scale_resources",
            Decision::InvestigateAnomaly(_) => "investigate_anomaly",
            Decision::Deploy(_) => "deploy",
            Decision::Optimize(_) => "optimize",
            Decision::Heal(_) => "heal",
            Decision::Learn(_) => "learn",
            Decision::Communicate(_) => "communicate",
        }
    }
    
    /// Get/set score (used for prioritization)
    pub fn score(&self) -> f64 {
        // This would be stored in the actual implementation
        0.5
    }
    
    pub fn set_score(&mut self, _score: f64) {
        // This would update the stored score
    }
}

/// Priority levels for decisions
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentDecision {
    pub id: String,
    pub component: String,
    pub version: String,
    pub strategy: DeploymentStrategy,
    pub priority: Priority,
    pub rollback_plan: Option<RollbackPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    BlueGreen,
    Canary { percentage: u8 },
    Rolling { batch_size: usize },
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub trigger_conditions: Vec<RollbackTrigger>,
    pub target_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackTrigger {
    ErrorRateAbove(f64),
    LatencyAbove(u64),
    HealthCheckFailure,
    ManualTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationDecision {
    pub id: String,
    pub target: OptimizationTarget,
    pub strategy: String,
    pub expected_improvement: f64,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationTarget {
    Performance,
    Cost,
    Resource,
    Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingDecision {
    pub id: String,
    pub component: String,
    pub issue: String,
    pub healing_action: HealingAction,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealingAction {
    Restart,
    Reconfigure,
    Repair,
    Replace,
    Isolate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningDecision {
    pub id: String,
    pub learning_type: LearningType,
    pub data_source: String,
    pub model: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningType {
    Supervised,
    Unsupervised,
    Reinforcement,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationDecision {
    pub id: String,
    pub target: CommunicationTarget,
    pub message_type: MessageType,
    pub content: serde_json::Value,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationTarget {
    User(String),
    System(String),
    Service(String),
    Team(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Alert,
    Report,
    Request,
    Response,
    Notification,
}