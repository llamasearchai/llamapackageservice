/// Execution engine for running workflows and operations
pub mod workflow_engine;
pub mod executors;
pub mod scheduling;
pub mod recovery;

use anyhow::Result;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::integrations::IntegrationManager;

pub use workflow_engine::{WorkflowEngine, Workflow, WorkflowTrigger};
pub use executors::{Executor, ExecutorType};
pub use scheduling::{Scheduler, Schedule};
pub use recovery::{RecoveryStrategy, RecoveryManager};

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfig {
    pub max_concurrent_workflows: usize,
    pub workflow_timeout_seconds: u64,
    pub enable_auto_recovery: bool,
    pub enable_distributed_execution: bool,
    pub executor_pool_size: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 50,
            workflow_timeout_seconds: 3600,
            enable_auto_recovery: true,
            enable_distributed_execution: false,
            executor_pool_size: 20,
        }
    }
}

/// Main execution engine
pub struct ExecutionEngine {
    config: ExecutionConfig,
    workflow_engine: Arc<WorkflowEngine>,
    scheduler: Arc<Scheduler>,
    recovery_manager: Arc<RecoveryManager>,
    integration_manager: Arc<IntegrationManager>,
}

impl ExecutionEngine {
    pub async fn new(
        config: &ExecutionConfig,
        integration_manager: Arc<IntegrationManager>,
    ) -> Result<Self> {
        tracing::info!("Initializing Execution Engine");
        
        // Initialize components
        let workflow_engine = Arc::new(
            WorkflowEngine::new(config, integration_manager.clone()).await?
        );
        
        let scheduler = Arc::new(
            Scheduler::new(workflow_engine.clone()).await?
        );
        
        let recovery_manager = Arc::new(
            RecoveryManager::new(config, workflow_engine.clone()).await?
        );
        
        Ok(Self {
            config: config.clone(),
            workflow_engine,
            scheduler,
            recovery_manager,
            integration_manager,
        })
    }
    
    /// Trigger a workflow
    pub async fn trigger_workflow(&self, workflow: Workflow) -> Result<String> {
        self.workflow_engine.trigger(workflow).await
    }
    
    /// Self-heal from an error
    pub async fn self_heal(&self, error: anyhow::Error) -> HealingResult {
        tracing::info!("Attempting self-healing for error: {:?}", error);
        
        // Analyze error
        let error_type = self.classify_error(&error);
        
        // Determine recovery strategy
        let strategy = self.recovery_manager.determine_strategy(&error_type).await;
        
        // Execute recovery
        match self.recovery_manager.execute_recovery(strategy).await {
            Ok(()) => HealingResult {
                success: true,
                actions_taken: vec!["Recovery completed".to_string()],
                error_type,
            },
            Err(e) => HealingResult {
                success: false,
                actions_taken: vec![format!("Recovery failed: {}", e)],
                error_type,
            },
        }
    }
    
    /// Start the execution engine
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting Execution Engine");
        
        // Start components
        tokio::try_join!(
            self.workflow_engine.start(),
            self.scheduler.start(),
            self.recovery_manager.start(),
        )?;
        
        Ok(())
    }
    
    /// Stop the execution engine
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping Execution Engine");
        
        // Stop components
        tokio::try_join!(
            self.workflow_engine.stop(),
            self.scheduler.stop(),
            self.recovery_manager.stop(),
        )?;
        
        Ok(())
    }
    
    /// Classify error type
    fn classify_error(&self, error: &anyhow::Error) -> ErrorType {
        let error_str = error.to_string().to_lowercase();
        
        if error_str.contains("timeout") {
            ErrorType::Timeout
        } else if error_str.contains("connection") || error_str.contains("network") {
            ErrorType::Network
        } else if error_str.contains("resource") || error_str.contains("memory") {
            ErrorType::Resource
        } else if error_str.contains("permission") || error_str.contains("auth") {
            ErrorType::Permission
        } else {
            ErrorType::Unknown
        }
    }
}

/// Result of self-healing attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingResult {
    pub success: bool,
    pub actions_taken: Vec<String>,
    pub error_type: ErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    Timeout,
    Network,
    Resource,
    Permission,
    Logic,
    Unknown,
}