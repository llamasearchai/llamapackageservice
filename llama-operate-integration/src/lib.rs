/// Llama-Operate Integration - Unified AI-powered development platform
pub mod bridge;
pub mod command_center;
pub mod workflows;
pub mod monitoring;
pub mod automation;
pub mod telemetry;

use anyhow::Result;
use std::sync::Arc;

/// Main integration coordinator
pub struct LlamaOperateSystem {
    bridge: Arc<bridge::PythonBridge>,
    command_center: Arc<command_center::CommandCenter>,
    workflow_engine: Arc<workflows::WorkflowEngine>,
    monitor: Arc<monitoring::SystemMonitor>,
    automation: Arc<automation::AutomationEngine>,
}

impl LlamaOperateSystem {
    pub async fn new(config: SystemConfig) -> Result<Self> {
        // Initialize Python bridge for Operate Enhanced
        let bridge = Arc::new(bridge::PythonBridge::new(&config.python_config).await?);
        
        // Initialize command center
        let command_center = Arc::new(
            command_center::CommandCenter::new(
                config.github_token.clone(),
                config.database_url.clone(),
            ).await?
        );
        
        // Initialize workflow engine
        let workflow_engine = Arc::new(
            workflows::WorkflowEngine::new(
                bridge.clone(),
                command_center.clone(),
            ).await?
        );
        
        // Initialize monitoring
        let monitor = Arc::new(
            monitoring::SystemMonitor::new(config.monitoring_config).await?
        );
        
        // Initialize automation
        let automation = Arc::new(
            automation::AutomationEngine::new(
                workflow_engine.clone(),
                monitor.clone(),
            ).await?
        );
        
        Ok(Self {
            bridge,
            command_center,
            workflow_engine,
            monitor,
            automation,
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        // Start all subsystems
        tokio::try_join!(
            self.monitor.start(),
            self.automation.start(),
            self.command_center.start_monitoring(),
        )?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SystemConfig {
    pub python_config: bridge::PythonConfig,
    pub github_token: String,
    pub database_url: String,
    pub monitoring_config: monitoring::MonitoringConfig,
    pub automation_rules: Vec<automation::Rule>,
}