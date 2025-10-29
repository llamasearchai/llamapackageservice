/// System context for decision making
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    integrations::IntegrationManager,
    agents::AgentOrchestrator,
    knowledge::KnowledgeBase,
};

/// Complete system context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Current timestamp
    pub timestamp: DateTime<Utc>,
    
    /// System health score (0.0 - 1.0)
    pub system_health: f64,
    
    /// Resource utilization
    pub resources: ResourceUtilization,
    
    /// Active tasks and their status
    pub active_tasks: Vec<TaskInfo>,
    
    /// Recent events
    pub recent_events: Vec<Event>,
    
    /// System metrics
    pub metrics: HashMap<String, MetricValue>,
    
    /// External integrations status
    pub integration_status: HashMap<String, IntegrationStatus>,
    
    /// Detected anomalies
    pub anomalies: Vec<super::Anomaly>,
    
    /// Environment variables
    pub environment: Environment,
    
    /// Predictions from learning system
    pub predictions: Vec<Prediction>,
}

impl Context {
    /// Gather context from all system components
    pub async fn gather(
        integration_manager: &IntegrationManager,
        agent_orchestrator: &AgentOrchestrator,
        knowledge_base: &KnowledgeBase,
    ) -> Result<Self> {
        // Gather resource utilization
        let resources = ResourceUtilization::current().await?;
        
        // Get active tasks
        let active_tasks = agent_orchestrator.get_active_tasks().await?;
        
        // Get recent events
        let recent_events = knowledge_base.get_recent_events(100).await?;
        
        // Collect metrics
        let metrics = Self::collect_metrics().await?;
        
        // Get integration status
        let integration_status = integration_manager.get_all_status().await?;
        
        // Check for anomalies
        let anomalies = Self::detect_anomalies(&metrics, &resources).await?;
        
        // Get environment info
        let environment = Environment::current();
        
        // Get predictions
        let predictions = knowledge_base.get_active_predictions().await?;
        
        // Calculate system health
        let system_health = Self::calculate_system_health(
            &resources,
            &active_tasks,
            &anomalies,
            &integration_status,
        );
        
        Ok(Self {
            timestamp: Utc::now(),
            system_health,
            resources,
            active_tasks,
            recent_events,
            metrics,
            integration_status,
            anomalies,
            environment,
            predictions,
        })
    }
    
    /// Check if optimization opportunities exist
    pub fn has_optimization_opportunities(&self) -> bool {
        // High resource usage
        if self.resources.cpu_usage > 80.0 || self.resources.memory_usage > 85.0 {
            return true;
        }
        
        // Slow response times
        if let Some(response_time) = self.metrics.get("avg_response_time") {
            if response_time.as_float() > 1000.0 {
                return true;
            }
        }
        
        // High error rate
        if let Some(error_rate) = self.metrics.get("error_rate") {
            if error_rate.as_float() > 5.0 {
                return true;
            }
        }
        
        false
    }
    
    /// Check if scaling is required
    pub fn requires_scaling(&self) -> bool {
        // Check CPU pressure
        if self.resources.cpu_usage > 90.0 {
            return true;
        }
        
        // Check memory pressure
        if self.resources.memory_usage > 90.0 {
            return true;
        }
        
        // Check task queue depth
        let pending_tasks = self.active_tasks.iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        
        if pending_tasks > 100 {
            return true;
        }
        
        false
    }
    
    /// Check if resources are available for a decision
    pub fn has_resources_for(&self, decision: &super::Decision) -> bool {
        match decision {
            super::Decision::ExecuteTask(task) => {
                // Check if we have capacity for more tasks
                self.active_tasks.len() < 1000 && self.resources.cpu_usage < 90.0
            },
            super::Decision::ScaleResources(req) => {
                // Always allow scaling decisions
                true
            },
            _ => true,
        }
    }
    
    /// Collect system metrics
    async fn collect_metrics() -> Result<HashMap<String, MetricValue>> {
        let mut metrics = HashMap::new();
        
        // Collect various metrics
        metrics.insert("avg_response_time".to_string(), MetricValue::Float(250.0));
        metrics.insert("error_rate".to_string(), MetricValue::Float(0.5));
        metrics.insert("requests_per_second".to_string(), MetricValue::Integer(1000));
        metrics.insert("active_connections".to_string(), MetricValue::Integer(500));
        
        Ok(metrics)
    }
    
    /// Detect anomalies
    async fn detect_anomalies(
        metrics: &HashMap<String, MetricValue>,
        resources: &ResourceUtilization,
    ) -> Result<Vec<super::Anomaly>> {
        let mut anomalies = Vec::new();
        
        // Check for high error rate
        if let Some(error_rate) = metrics.get("error_rate") {
            if error_rate.as_float() > 10.0 {
                anomalies.push(super::Anomaly {
                    id: uuid::Uuid::new_v4().to_string(),
                    anomaly_type: super::AnomalyType::Reliability,
                    severity: super::Severity::High,
                    description: "High error rate detected".to_string(),
                    detected_at: Utc::now(),
                    affected_components: vec!["api".to_string()],
                    metrics: serde_json::json!({
                        "error_rate": error_rate.as_float(),
                    }),
                });
            }
        }
        
        // Check for resource exhaustion
        if resources.memory_usage > 95.0 {
            anomalies.push(super::Anomaly {
                id: uuid::Uuid::new_v4().to_string(),
                anomaly_type: super::AnomalyType::Performance,
                severity: super::Severity::Critical,
                description: "Memory exhaustion imminent".to_string(),
                detected_at: Utc::now(),
                affected_components: vec!["system".to_string()],
                metrics: serde_json::json!({
                    "memory_usage": resources.memory_usage,
                }),
            });
        }
        
        Ok(anomalies)
    }
    
    /// Calculate overall system health
    fn calculate_system_health(
        resources: &ResourceUtilization,
        active_tasks: &[TaskInfo],
        anomalies: &[super::Anomaly],
        integration_status: &HashMap<String, IntegrationStatus>,
    ) -> f64 {
        let mut health = 1.0;
        
        // Factor in resource usage
        health -= (resources.cpu_usage / 100.0) * 0.2;
        health -= (resources.memory_usage / 100.0) * 0.2;
        
        // Factor in failed tasks
        let failed_tasks = active_tasks.iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count() as f64;
        let task_failure_rate = failed_tasks / active_tasks.len().max(1) as f64;
        health -= task_failure_rate * 0.3;
        
        // Factor in anomalies
        for anomaly in anomalies {
            match anomaly.severity {
                super::Severity::Critical => health -= 0.3,
                super::Severity::High => health -= 0.2,
                super::Severity::Medium => health -= 0.1,
                super::Severity::Low => health -= 0.05,
            }
        }
        
        // Factor in integration health
        let unhealthy_integrations = integration_status.values()
            .filter(|s| !s.healthy)
            .count() as f64;
        let integration_health = 1.0 - (unhealthy_integrations / integration_status.len().max(1) as f64);
        health *= integration_health;
        
        health.max(0.0).min(1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_usage: f64,
    pub gpu_usage: Option<f64>,
}

impl ResourceUtilization {
    async fn current() -> Result<Self> {
        // In production, this would use actual system metrics
        Ok(Self {
            cpu_usage: 45.0,
            memory_usage: 60.0,
            disk_usage: 70.0,
            network_usage: 30.0,
            gpu_usage: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub priority: super::Priority,
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl MetricValue {
    pub fn as_float(&self) -> f64 {
        match self {
            MetricValue::Integer(i) => *i as f64,
            MetricValue::Float(f) => *f,
            MetricValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub name: String,
    pub healthy: bool,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub deployment_type: String,
    pub region: String,
    pub cluster: String,
    pub version: String,
    pub features: Vec<String>,
}

impl Environment {
    fn current() -> Self {
        Self {
            deployment_type: "production".to_string(),
            region: "us-west-2".to_string(),
            cluster: "main".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            features: vec!["auto-scaling".to_string(), "ml-optimization".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub prediction_type: String,
    pub confidence: f64,
    pub predicted_at: DateTime<Utc>,
    pub time_horizon: chrono::Duration,
    pub data: serde_json::Value,
}