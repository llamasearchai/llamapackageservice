/// Self-healing and auto-optimization engine
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use chrono::{DateTime, Utc, Duration};

use crate::workflows::{WorkflowEngine, WorkflowContext, WorkflowTrigger};
use crate::monitoring::{SystemMonitor, Alert, AlertLevel};

pub struct AutomationEngine {
    workflow_engine: Arc<WorkflowEngine>,
    monitor: Arc<SystemMonitor>,
    rules: Arc<DashMap<String, Rule>>,
    active_remediations: Arc<DashMap<String, Remediation>>,
}

impl AutomationEngine {
    pub async fn new(
        workflow_engine: Arc<WorkflowEngine>,
        monitor: Arc<SystemMonitor>,
    ) -> Result<Self> {
        let engine = Self {
            workflow_engine,
            monitor,
            rules: Arc::new(DashMap::new()),
            active_remediations: Arc::new(DashMap::new()),
        };
        
        // Load default automation rules
        engine.load_default_rules().await?;
        
        Ok(engine)
    }
    
    async fn load_default_rules(&self) -> Result<()> {
        // Performance optimization rule
        self.register_rule(Rule {
            id: "perf_optimization".to_string(),
            name: "Performance Optimization".to_string(),
            description: "Automatically optimize slow code".to_string(),
            trigger: RuleTrigger::Metric {
                name: "response_time".to_string(),
                condition: MetricCondition::GreaterThan(1000.0),
                duration_minutes: 5,
            },
            action: RuleAction::TriggerWorkflow {
                workflow_id: "performance_optimization".to_string(),
            },
            cooldown_minutes: 60,
            enabled: true,
        })?;
        
        // Memory leak detection
        self.register_rule(Rule {
            id: "memory_leak".to_string(),
            name: "Memory Leak Detection".to_string(),
            description: "Detect and fix memory leaks".to_string(),
            trigger: RuleTrigger::Metric {
                name: "memory_usage".to_string(),
                condition: MetricCondition::TrendIncreasing {
                    threshold: 5.0,
                    window_minutes: 30,
                },
                duration_minutes: 10,
            },
            action: RuleAction::Composite(vec![
                RuleAction::CreateAlert {
                    level: AlertLevel::Warning,
                    message: "Potential memory leak detected".to_string(),
                },
                RuleAction::TriggerWorkflow {
                    workflow_id: "memory_analysis".to_string(),
                },
            ]),
            cooldown_minutes: 120,
            enabled: true,
        })?;
        
        // Failed deployment recovery
        self.register_rule(Rule {
            id: "deployment_recovery".to_string(),
            name: "Failed Deployment Recovery".to_string(),
            description: "Automatically rollback failed deployments".to_string(),
            trigger: RuleTrigger::Alert {
                pattern: "deployment.*failed".to_string(),
                level: Some(AlertLevel::Error),
            },
            action: RuleAction::ExecuteRemediation {
                remediation_type: RemediationType::Rollback,
                parameters: serde_json::json!({
                    "target": "last_stable",
                    "verify": true,
                }),
            },
            cooldown_minutes: 30,
            enabled: true,
        })?;
        
        // Test failure analysis
        self.register_rule(Rule {
            id: "test_failure".to_string(),
            name: "Test Failure Analysis".to_string(),
            description: "Analyze and fix test failures".to_string(),
            trigger: RuleTrigger::Event {
                event_type: "test_failure".to_string(),
                filter: Some(serde_json::json!({
                    "severity": "high",
                })),
            },
            action: RuleAction::TriggerWorkflow {
                workflow_id: "test_fix".to_string(),
            },
            cooldown_minutes: 15,
            enabled: true,
        })?;
        
        // Resource optimization
        self.register_rule(Rule {
            id: "resource_optimization".to_string(),
            name: "Resource Optimization".to_string(),
            description: "Optimize resource usage".to_string(),
            trigger: RuleTrigger::Schedule {
                cron: "0 2 * * *".to_string(), // Daily at 2 AM
            },
            action: RuleAction::Composite(vec![
                RuleAction::AnalyzeResources,
                RuleAction::TriggerWorkflow {
                    workflow_id: "resource_optimization".to_string(),
                },
            ]),
            cooldown_minutes: 1440, // 24 hours
            enabled: true,
        })?;
        
        Ok(())
    }
    
    pub fn register_rule(&self, rule: Rule) -> Result<()> {
        self.rules.insert(rule.id.clone(), rule);
        Ok(())
    }
    
    pub async fn start(&self) -> Result<()> {
        // Start rule evaluation loop
        let rules_handle = self.start_rule_evaluation();
        
        // Start remediation executor
        let remediation_handle = self.start_remediation_executor();
        
        // Start optimization loop
        let optimization_handle = self.start_optimization_loop();
        
        tokio::try_join!(rules_handle, remediation_handle, optimization_handle)?;
        
        Ok(())
    }
    
    async fn start_rule_evaluation(&self) -> Result<()> {
        let rules = self.rules.clone();
        let monitor = self.monitor.clone();
        let workflow_engine = self.workflow_engine.clone();
        let active_remediations = self.active_remediations.clone();
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
            let mut last_triggered: DashMap<String, DateTime<Utc>> = DashMap::new();
            
            loop {
                ticker.tick().await;
                
                for entry in rules.iter() {
                    let rule = entry.value();
                    
                    if !rule.enabled {
                        continue;
                    }
                    
                    // Check cooldown
                    if let Some(last_time) = last_triggered.get(&rule.id) {
                        if Utc::now() - *last_time < Duration::minutes(rule.cooldown_minutes as i64) {
                            continue;
                        }
                    }
                    
                    // Evaluate trigger
                    let should_trigger = match &rule.trigger {
                        RuleTrigger::Metric { name, condition, duration_minutes } => {
                            evaluate_metric_trigger(&monitor, name, condition, *duration_minutes).await
                        },
                        RuleTrigger::Alert { pattern, level } => {
                            evaluate_alert_trigger(&monitor, pattern, level).await
                        },
                        RuleTrigger::Event { event_type, filter } => {
                            evaluate_event_trigger(event_type, filter).await
                        },
                        RuleTrigger::Schedule { cron } => {
                            evaluate_schedule_trigger(cron).await
                        },
                    };
                    
                    if should_trigger {
                        tracing::info!("Triggering automation rule: {}", rule.name);
                        
                        // Execute action
                        if let Err(e) = execute_rule_action(
                            &rule.action,
                            &rule,
                            &workflow_engine,
                            &monitor,
                            &active_remediations,
                        ).await {
                            tracing::error!("Failed to execute rule action: {}", e);
                        }
                        
                        last_triggered.insert(rule.id.clone(), Utc::now());
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn start_remediation_executor(&self) -> Result<()> {
        let active_remediations = self.active_remediations.clone();
        let workflow_engine = self.workflow_engine.clone();
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
            
            loop {
                ticker.tick().await;
                
                let mut completed = Vec::new();
                
                for entry in active_remediations.iter() {
                    let remediation = entry.value();
                    
                    match &remediation.status {
                        RemediationStatus::Pending => {
                            // Start remediation
                            if let Err(e) = execute_remediation(remediation, &workflow_engine).await {
                                tracing::error!("Remediation failed: {}", e);
                                
                                let mut updated = remediation.clone();
                                updated.status = RemediationStatus::Failed;
                                updated.error = Some(e.to_string());
                                active_remediations.insert(remediation.id.clone(), updated);
                            }
                        },
                        RemediationStatus::Running => {
                            // Check progress
                            // Implementation would check workflow status
                        },
                        RemediationStatus::Completed | RemediationStatus::Failed => {
                            // Mark for removal
                            completed.push(remediation.id.clone());
                        },
                    }
                }
                
                // Remove completed remediations
                for id in completed {
                    active_remediations.remove(&id);
                }
            }
        });
        
        Ok(())
    }
    
    async fn start_optimization_loop(&self) -> Result<()> {
        let monitor = self.monitor.clone();
        let workflow_engine = self.workflow_engine.clone();
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
            
            loop {
                ticker.tick().await;
                
                // Get system metrics
                let cpu_metrics = monitor.get_metrics("system.cpu", Duration::minutes(30)).await;
                let memory_metrics = monitor.get_metrics("system.memory", Duration::minutes(30)).await;
                
                // Analyze for optimization opportunities
                let optimizations = analyze_optimization_opportunities(&cpu_metrics, &memory_metrics);
                
                for opt in optimizations {
                    tracing::info!("Found optimization opportunity: {}", opt.description);
                    
                    // Trigger optimization workflow
                    if let Err(e) = workflow_engine.trigger_workflow(
                        &opt.workflow_id,
                        WorkflowContext {
                            repository: "system".to_string(),
                            trigger: WorkflowTrigger::Manual,
                            metadata: serde_json::json!({
                                "optimization": opt,
                            }),
                        },
                    ).await {
                        tracing::error!("Failed to trigger optimization workflow: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: RuleTrigger,
    pub action: RuleAction,
    pub cooldown_minutes: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleTrigger {
    Metric {
        name: String,
        condition: MetricCondition,
        duration_minutes: u32,
    },
    Alert {
        pattern: String,
        level: Option<AlertLevel>,
    },
    Event {
        event_type: String,
        filter: Option<serde_json::Value>,
    },
    Schedule {
        cron: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricCondition {
    GreaterThan(f64),
    LessThan(f64),
    Equals(f64),
    TrendIncreasing {
        threshold: f64,
        window_minutes: u32,
    },
    TrendDecreasing {
        threshold: f64,
        window_minutes: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    TriggerWorkflow {
        workflow_id: String,
    },
    CreateAlert {
        level: AlertLevel,
        message: String,
    },
    ExecuteRemediation {
        remediation_type: RemediationType,
        parameters: serde_json::Value,
    },
    AnalyzeResources,
    Composite(Vec<RuleAction>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationType {
    Restart,
    Rollback,
    ScaleUp,
    ScaleDown,
    Repair,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remediation {
    pub id: String,
    pub remediation_type: RemediationType,
    pub target: String,
    pub parameters: serde_json::Value,
    pub status: RemediationStatus,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizationOpportunity {
    pub id: String,
    pub description: String,
    pub impact: f64,
    pub workflow_id: String,
    pub parameters: serde_json::Value,
}

// Helper functions
async fn evaluate_metric_trigger(
    monitor: &SystemMonitor,
    metric_name: &str,
    condition: &MetricCondition,
    duration_minutes: u32,
) -> bool {
    let metrics = monitor.get_metrics(metric_name, Duration::minutes(duration_minutes as i64)).await;
    
    if metrics.is_empty() {
        return false;
    }
    
    match condition {
        MetricCondition::GreaterThan(threshold) => {
            metrics.iter().all(|m| m.value > *threshold)
        },
        MetricCondition::LessThan(threshold) => {
            metrics.iter().all(|m| m.value < *threshold)
        },
        MetricCondition::Equals(value) => {
            metrics.iter().all(|m| (m.value - value).abs() < 0.01)
        },
        MetricCondition::TrendIncreasing { threshold, window_minutes } => {
            // Simple trend detection
            if metrics.len() < 2 {
                return false;
            }
            
            let first = metrics.first().unwrap().value;
            let last = metrics.last().unwrap().value;
            let increase_percent = ((last - first) / first) * 100.0;
            
            increase_percent > *threshold
        },
        MetricCondition::TrendDecreasing { threshold, window_minutes } => {
            if metrics.len() < 2 {
                return false;
            }
            
            let first = metrics.first().unwrap().value;
            let last = metrics.last().unwrap().value;
            let decrease_percent = ((first - last) / first) * 100.0;
            
            decrease_percent > *threshold
        },
    }
}

async fn evaluate_alert_trigger(
    monitor: &SystemMonitor,
    pattern: &str,
    level: &Option<AlertLevel>,
) -> bool {
    // Implementation would check active alerts
    false
}

async fn evaluate_event_trigger(
    event_type: &str,
    filter: &Option<serde_json::Value>,
) -> bool {
    // Implementation would check event stream
    false
}

async fn evaluate_schedule_trigger(cron: &str) -> bool {
    // Implementation would check cron schedule
    false
}

async fn execute_rule_action(
    action: &RuleAction,
    rule: &Rule,
    workflow_engine: &WorkflowEngine,
    monitor: &SystemMonitor,
    active_remediations: &DashMap<String, Remediation>,
) -> Result<()> {
    match action {
        RuleAction::TriggerWorkflow { workflow_id } => {
            workflow_engine.trigger_workflow(
                workflow_id,
                WorkflowContext {
                    repository: "system".to_string(),
                    trigger: WorkflowTrigger::Manual,
                    metadata: serde_json::json!({
                        "rule_id": rule.id,
                        "rule_name": rule.name,
                    }),
                },
            ).await?;
        },
        RuleAction::CreateAlert { level, message } => {
            monitor.create_alert(Alert {
                id: uuid::Uuid::new_v4().to_string(),
                level: *level,
                source: format!("automation.{}", rule.id),
                message: message.clone(),
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "rule_id": rule.id,
                }),
            }).await;
        },
        RuleAction::ExecuteRemediation { remediation_type, parameters } => {
            let remediation = Remediation {
                id: uuid::Uuid::new_v4().to_string(),
                remediation_type: remediation_type.clone(),
                target: "system".to_string(),
                parameters: parameters.clone(),
                status: RemediationStatus::Pending,
                created_at: Utc::now(),
                error: None,
            };
            
            active_remediations.insert(remediation.id.clone(), remediation);
        },
        RuleAction::AnalyzeResources => {
            // Trigger resource analysis
            workflow_engine.trigger_workflow(
                "resource_analysis",
                WorkflowContext {
                    repository: "system".to_string(),
                    trigger: WorkflowTrigger::Manual,
                    metadata: serde_json::json!({}),
                },
            ).await?;
        },
        RuleAction::Composite(actions) => {
            for sub_action in actions {
                execute_rule_action(sub_action, rule, workflow_engine, monitor, active_remediations).await?;
            }
        },
    }
    
    Ok(())
}

async fn execute_remediation(
    remediation: &Remediation,
    workflow_engine: &WorkflowEngine,
) -> Result<()> {
    match &remediation.remediation_type {
        RemediationType::Rollback => {
            workflow_engine.trigger_workflow(
                "rollback",
                WorkflowContext {
                    repository: remediation.target.clone(),
                    trigger: WorkflowTrigger::Manual,
                    metadata: remediation.parameters.clone(),
                },
            ).await?;
        },
        _ => {
            // Implementation for other remediation types
        }
    }
    
    Ok(())
}

fn analyze_optimization_opportunities(
    cpu_metrics: &[crate::monitoring::MetricPoint],
    memory_metrics: &[crate::monitoring::MetricPoint],
) -> Vec<OptimizationOpportunity> {
    let mut opportunities = Vec::new();
    
    // Check for high CPU usage
    if let Some(avg_cpu) = calculate_average(cpu_metrics) {
        if avg_cpu > 80.0 {
            opportunities.push(OptimizationOpportunity {
                id: uuid::Uuid::new_v4().to_string(),
                description: "High CPU usage detected".to_string(),
                impact: (avg_cpu - 50.0) / 50.0,
                workflow_id: "cpu_optimization".to_string(),
                parameters: serde_json::json!({
                    "current_usage": avg_cpu,
                }),
            });
        }
    }
    
    // Check for memory issues
    if let Some(avg_memory) = calculate_average(memory_metrics) {
        if avg_memory > 85.0 {
            opportunities.push(OptimizationOpportunity {
                id: uuid::Uuid::new_v4().to_string(),
                description: "High memory usage detected".to_string(),
                impact: (avg_memory - 60.0) / 40.0,
                workflow_id: "memory_optimization".to_string(),
                parameters: serde_json::json!({
                    "current_usage": avg_memory,
                }),
            });
        }
    }
    
    opportunities
}

fn calculate_average(metrics: &[crate::monitoring::MetricPoint]) -> Option<f64> {
    if metrics.is_empty() {
        return None;
    }
    
    let sum: f64 = metrics.iter().map(|m| m.value).sum();
    Some(sum / metrics.len() as f64)
}

// Add uuid to dependencies
use uuid;