/// Intelligent workflow engine for automated development tasks
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use async_trait::async_trait;

use crate::bridge::{PythonBridge, Operation, GitHubOperation};
use crate::command_center::{CommandCenter, Task, TaskRequest};

pub struct WorkflowEngine {
    bridge: Arc<PythonBridge>,
    command_center: Arc<CommandCenter>,
    workflows: Arc<DashMap<String, WorkflowDefinition>>,
    running_workflows: Arc<DashMap<String, WorkflowInstance>>,
}

impl WorkflowEngine {
    pub async fn new(
        bridge: Arc<PythonBridge>,
        command_center: Arc<CommandCenter>,
    ) -> Result<Self> {
        let engine = Self {
            bridge,
            command_center,
            workflows: Arc::new(DashMap::new()),
            running_workflows: Arc::new(DashMap::new()),
        };
        
        // Load built-in workflows
        engine.load_builtin_workflows().await?;
        
        Ok(engine)
    }
    
    async fn load_builtin_workflows(&self) -> Result<()> {
        // Code Review Workflow
        self.register_workflow(WorkflowDefinition {
            id: "code_review".to_string(),
            name: "Automated Code Review".to_string(),
            description: "Analyzes PRs and provides feedback".to_string(),
            triggers: vec![
                WorkflowTrigger::GitHubEvent("pull_request.opened".to_string()),
                WorkflowTrigger::GitHubEvent("pull_request.synchronize".to_string()),
            ],
            steps: vec![
                WorkflowStep {
                    id: "analyze_changes".to_string(),
                    step_type: StepType::AnalyzeCode,
                    inputs: serde_json::json!({
                        "focus": "changes",
                        "checks": ["security", "performance", "style"]
                    }),
                    on_success: Some("post_review".to_string()),
                    on_failure: Some("notify_error".to_string()),
                },
                WorkflowStep {
                    id: "post_review".to_string(),
                    step_type: StepType::PostComment,
                    inputs: serde_json::json!({
                        "template": "code_review"
                    }),
                    on_success: None,
                    on_failure: Some("notify_error".to_string()),
                },
                WorkflowStep {
                    id: "notify_error".to_string(),
                    step_type: StepType::Notify,
                    inputs: serde_json::json!({
                        "channel": "errors",
                        "level": "warning"
                    }),
                    on_success: None,
                    on_failure: None,
                },
            ],
        })?;
        
        // Bug Fix Workflow
        self.register_workflow(WorkflowDefinition {
            id: "bug_fix".to_string(),
            name: "Automated Bug Fix".to_string(),
            description: "Attempts to fix reported bugs automatically".to_string(),
            triggers: vec![
                WorkflowTrigger::GitHubEvent("issues.labeled".to_string()),
                WorkflowTrigger::Manual,
            ],
            steps: vec![
                WorkflowStep {
                    id: "analyze_issue".to_string(),
                    step_type: StepType::AnalyzeIssue,
                    inputs: serde_json::json!({}),
                    on_success: Some("reproduce_bug".to_string()),
                    on_failure: Some("request_clarification".to_string()),
                },
                WorkflowStep {
                    id: "reproduce_bug".to_string(),
                    step_type: StepType::RunTests,
                    inputs: serde_json::json!({
                        "type": "reproduction"
                    }),
                    on_success: Some("generate_fix".to_string()),
                    on_failure: Some("request_clarification".to_string()),
                },
                WorkflowStep {
                    id: "generate_fix".to_string(),
                    step_type: StepType::GenerateCode,
                    inputs: serde_json::json!({
                        "type": "bug_fix"
                    }),
                    on_success: Some("test_fix".to_string()),
                    on_failure: Some("escalate".to_string()),
                },
                WorkflowStep {
                    id: "test_fix".to_string(),
                    step_type: StepType::RunTests,
                    inputs: serde_json::json!({
                        "type": "validation"
                    }),
                    on_success: Some("create_pr".to_string()),
                    on_failure: Some("iterate_fix".to_string()),
                },
                WorkflowStep {
                    id: "create_pr".to_string(),
                    step_type: StepType::CreatePR,
                    inputs: serde_json::json!({
                        "template": "bug_fix"
                    }),
                    on_success: None,
                    on_failure: Some("escalate".to_string()),
                },
            ],
        })?;
        
        // Continuous Improvement Workflow
        self.register_workflow(WorkflowDefinition {
            id: "continuous_improvement".to_string(),
            name: "Continuous Code Improvement".to_string(),
            description: "Regularly improves code quality".to_string(),
            triggers: vec![
                WorkflowTrigger::Schedule("0 0 * * *".to_string()), // Daily
            ],
            steps: vec![
                WorkflowStep {
                    id: "scan_codebase".to_string(),
                    step_type: StepType::AnalyzeCode,
                    inputs: serde_json::json!({
                        "focus": "quality",
                        "checks": ["complexity", "duplication", "dependencies"]
                    }),
                    on_success: Some("prioritize_improvements".to_string()),
                    on_failure: None,
                },
                WorkflowStep {
                    id: "prioritize_improvements".to_string(),
                    step_type: StepType::Prioritize,
                    inputs: serde_json::json!({
                        "criteria": ["impact", "effort", "risk"]
                    }),
                    on_success: Some("implement_improvements".to_string()),
                    on_failure: None,
                },
                WorkflowStep {
                    id: "implement_improvements".to_string(),
                    step_type: StepType::GenerateCode,
                    inputs: serde_json::json!({
                        "type": "refactoring",
                        "limit": 5
                    }),
                    on_success: Some("validate_improvements".to_string()),
                    on_failure: None,
                },
                WorkflowStep {
                    id: "validate_improvements".to_string(),
                    step_type: StepType::RunTests,
                    inputs: serde_json::json!({
                        "type": "full"
                    }),
                    on_success: Some("create_improvement_pr".to_string()),
                    on_failure: None,
                },
            ],
        })?;
        
        Ok(())
    }
    
    pub fn register_workflow(&self, workflow: WorkflowDefinition) -> Result<()> {
        self.workflows.insert(workflow.id.clone(), workflow);
        Ok(())
    }
    
    pub async fn trigger_workflow(
        &self,
        workflow_id: &str,
        context: WorkflowContext,
    ) -> Result<String> {
        let workflow = self.workflows.get(workflow_id)
            .context("Workflow not found")?
            .clone();
        
        let instance = WorkflowInstance {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id: workflow.id.clone(),
            status: WorkflowStatus::Running,
            current_step: 0,
            context,
            results: Vec::new(),
        };
        
        let instance_id = instance.id.clone();
        self.running_workflows.insert(instance_id.clone(), instance);
        
        // Execute workflow in background
        let engine = Arc::new(self.clone());
        tokio::spawn(async move {
            if let Err(e) = engine.execute_workflow(&instance_id).await {
                tracing::error!("Workflow execution failed: {}", e);
            }
        });
        
        Ok(instance_id)
    }
    
    async fn execute_workflow(&self, instance_id: &str) -> Result<()> {
        loop {
            let (workflow, mut instance) = {
                let instance = self.running_workflows.get(instance_id)
                    .context("Workflow instance not found")?;
                
                let workflow = self.workflows.get(&instance.workflow_id)
                    .context("Workflow definition not found")?;
                
                (workflow.clone(), instance.clone())
            };
            
            if instance.current_step >= workflow.steps.len() {
                instance.status = WorkflowStatus::Completed;
                self.running_workflows.insert(instance_id.to_string(), instance);
                break;
            }
            
            let step = &workflow.steps[instance.current_step];
            let result = self.execute_step(step, &instance.context).await;
            
            match result {
                Ok(step_result) => {
                    instance.results.push(step_result.clone());
                    
                    let next_step = if step_result.success {
                        step.on_success.as_ref()
                    } else {
                        step.on_failure.as_ref()
                    };
                    
                    if let Some(next_step_id) = next_step {
                        // Find next step index
                        if let Some(next_index) = workflow.steps.iter()
                            .position(|s| &s.id == next_step_id) {
                            instance.current_step = next_index;
                        } else {
                            instance.current_step += 1;
                        }
                    } else {
                        instance.current_step = workflow.steps.len(); // End workflow
                    }
                },
                Err(e) => {
                    tracing::error!("Step execution failed: {}", e);
                    instance.status = WorkflowStatus::Failed;
                    self.running_workflows.insert(instance_id.to_string(), instance);
                    return Err(e);
                }
            }
            
            self.running_workflows.insert(instance_id.to_string(), instance);
        }
        
        Ok(())
    }
    
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        context: &WorkflowContext,
    ) -> Result<StepResult> {
        match &step.step_type {
            StepType::AnalyzeCode => {
                let task = self.command_center.create_task(TaskRequest {
                    task_type: "analyze_code".to_string(),
                    repository: context.repository.clone(),
                    title: "Code analysis".to_string(),
                    description: step.inputs.to_string(),
                }).await?;
                
                let result = self.command_center.execute_task(&task.id).await?;
                
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: result.success,
                    output: result.data,
                })
            },
            StepType::RunTests => {
                // Use Operate Enhanced to run tests
                let operation = Operation {
                    id: uuid::Uuid::new_v4().to_string(),
                    action_type: "execute".to_string(),
                    target: Some("terminal".to_string()),
                    value: Some(serde_json::json!("cargo test")),
                    metadata: serde_json::json!({}),
                };
                
                let result = self.bridge.execute_operation(operation).await?;
                
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: result.status == "success",
                    output: result.data,
                })
            },
            StepType::GenerateCode => {
                // Use AI to generate code fixes
                let objective = format!(
                    "Generate {} for repository {}",
                    step.inputs["type"].as_str().unwrap_or("code"),
                    context.repository
                );
                
                let action = self.bridge.analyze_screen(&objective).await?;
                
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: Some(serde_json::to_value(action)?),
                })
            },
            StepType::CreatePR => {
                let pr_result = self.bridge.github_operation(
                    GitHubOperation::CreatePR {
                        repo: context.repository.clone(),
                        title: "Automated improvement".to_string(),
                        body: "Generated by workflow".to_string(),
                        head: "auto-fix".to_string(),
                        base: "main".to_string(),
                    }
                ).await?;
                
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: Some(pr_result),
                })
            },
            _ => {
                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: None,
                })
            }
        }
    }
}

// Clone implementation for WorkflowEngine
impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            bridge: self.bridge.clone(),
            command_center: self.command_center.clone(),
            workflows: self.workflows.clone(),
            running_workflows: self.running_workflows.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub triggers: Vec<WorkflowTrigger>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTrigger {
    GitHubEvent(String),
    Schedule(String),
    Manual,
    Webhook(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub step_type: StepType,
    pub inputs: serde_json::Value,
    pub on_success: Option<String>,
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    AnalyzeCode,
    AnalyzeIssue,
    RunTests,
    GenerateCode,
    CreatePR,
    PostComment,
    Notify,
    Prioritize,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub current_step: usize,
    pub context: WorkflowContext,
    pub results: Vec<StepResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub repository: String,
    pub trigger: WorkflowTrigger,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
}