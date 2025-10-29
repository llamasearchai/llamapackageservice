use llama_operate::{LlamaOperateSystem, SystemConfig};
use llama_operate::command_center::{Repository, TaskRequest};
use llama_operate::workflows::{WorkflowContext, WorkflowTrigger};
use tempfile::TempDir;

async fn setup_test_system() -> (LlamaOperateSystem, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    let config = SystemConfig {
        python_config: llama_operate::bridge::PythonConfig {
            operate_path: "../operate-enhanced".to_string(),
            model: "gpt-4o".to_string(),
            sandbox_mode: true,
            cache_enabled: false,
        },
        github_token: "test_token".to_string(),
        database_url: format!("sqlite://{}", db_path.display()),
        monitoring_config: llama_operate::monitoring::MonitoringConfig::default(),
        automation_rules: vec![],
    };
    
    let system = LlamaOperateSystem::new(config).await.unwrap();
    (system, temp_dir)
}

#[tokio::test]
async fn test_repository_management() {
    let (system, _temp) = setup_test_system().await;
    
    // Register repository
    let repo = Repository {
        full_name: "test/repo".to_string(),
        owner: "test".to_string(),
        name: "repo".to_string(),
        description: Some("Test repository".to_string()),
        language: Some("Rust".to_string()),
        stars: 100,
        last_sync: chrono::Utc::now(),
    };
    
    system.command_center.register_repository(repo.clone()).await.unwrap();
    
    // Verify repository was registered
    // In a real test, we would query the database
}

#[tokio::test]
async fn test_task_creation_and_execution() {
    let (system, _temp) = setup_test_system().await;
    
    // Create task
    let task_request = TaskRequest {
        task_type: "analyze_code".to_string(),
        repository: "test/repo".to_string(),
        title: "Test analysis".to_string(),
        description: "Test task".to_string(),
    };
    
    let task = system.command_center.create_task(task_request).await.unwrap();
    
    assert_eq!(task.task_type, "analyze_code");
    assert_eq!(task.repository, "test/repo");
    assert_eq!(task.status, llama_operate::command_center::TaskStatus::Pending);
}

#[tokio::test]
async fn test_workflow_trigger() {
    let (system, _temp) = setup_test_system().await;
    
    // Trigger workflow
    let context = WorkflowContext {
        repository: "test/repo".to_string(),
        trigger: WorkflowTrigger::Manual,
        metadata: serde_json::json!({}),
    };
    
    let instance_id = system.workflow_engine
        .trigger_workflow("code_review", context)
        .await
        .unwrap();
    
    assert!(!instance_id.is_empty());
}

#[tokio::test]
async fn test_monitoring_metrics() {
    let (system, _temp) = setup_test_system().await;
    
    // Record metrics
    system.monitor.record_metric("test.metric", 42.0).await;
    system.monitor.record_metric("test.metric", 43.0).await;
    system.monitor.record_metric("test.metric", 44.0).await;
    
    // Get metrics
    let metrics = system.monitor
        .get_metrics("test.metric", chrono::Duration::minutes(5))
        .await;
    
    assert_eq!(metrics.len(), 3);
    assert_eq!(metrics[0].value, 42.0);
}

#[tokio::test]
async fn test_alert_creation() {
    let (system, _temp) = setup_test_system().await;
    
    // Create alert
    let alert = llama_operate::monitoring::Alert {
        id: "test_alert".to_string(),
        level: llama_operate::monitoring::AlertLevel::Warning,
        source: "test".to_string(),
        message: "Test alert".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: serde_json::json!({}),
    };
    
    system.monitor.create_alert(alert).await;
    
    // Get system status
    let status = system.monitor.get_system_status().await;
    assert!(status.active_alerts > 0);
}

#[tokio::test]
async fn test_automation_rule_registration() {
    let (system, _temp) = setup_test_system().await;
    
    // Register rule
    let rule = llama_operate::automation::Rule {
        id: "test_rule".to_string(),
        name: "Test Rule".to_string(),
        description: "Test automation rule".to_string(),
        trigger: llama_operate::automation::RuleTrigger::Metric {
            name: "test.metric".to_string(),
            condition: llama_operate::automation::MetricCondition::GreaterThan(100.0),
            duration_minutes: 5,
        },
        action: llama_operate::automation::RuleAction::CreateAlert {
            level: llama_operate::monitoring::AlertLevel::Warning,
            message: "Test alert from rule".to_string(),
        },
        cooldown_minutes: 60,
        enabled: true,
    };
    
    system.automation.register_rule(rule).unwrap();
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_python_bridge_operation() {
        let (system, _temp) = setup_test_system().await;
        
        let operation = llama_operate::bridge::Operation {
            id: "test_op".to_string(),
            action_type: "screenshot".to_string(),
            target: None,
            value: None,
            metadata: serde_json::json!({}),
        };
        
        let result = system.bridge.execute_operation(operation).await.unwrap();
        assert_eq!(result.status, "success");
    }
}