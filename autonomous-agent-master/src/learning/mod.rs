/// Machine learning and adaptation system
pub mod models;
pub mod training;
pub mod inference;
pub mod adaptation;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::{
    knowledge::KnowledgeBase,
    core::Context,
    execution::HealingResult,
};

#[derive(Debug, Clone, Deserialize)]
pub struct LearningConfig {
    pub model_type: models::ModelType,
    pub training_interval_hours: u64,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub enable_online_learning: bool,
    pub enable_transfer_learning: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            model_type: models::ModelType::Transformer,
            training_interval_hours: 24,
            batch_size: 32,
            learning_rate: 0.001,
            enable_online_learning: true,
            enable_transfer_learning: true,
        }
    }
}

/// Learning system for continuous improvement
pub struct LearningSystem {
    config: LearningConfig,
    model_manager: Arc<models::ModelManager>,
    trainer: Arc<training::Trainer>,
    inference_engine: Arc<inference::InferenceEngine>,
    adaptation_engine: Arc<adaptation::AdaptationEngine>,
    knowledge_base: Arc<KnowledgeBase>,
    learning_history: Arc<RwLock<Vec<LearningEvent>>>,
}

impl LearningSystem {
    pub async fn new(
        config: &LearningConfig,
        knowledge_base: Arc<KnowledgeBase>,
    ) -> Result<Self> {
        tracing::info!("Initializing Learning System");
        
        // Initialize components
        let model_manager = Arc::new(models::ModelManager::new(&config.model_type).await?);
        let trainer = Arc::new(training::Trainer::new(config, model_manager.clone()).await?);
        let inference_engine = Arc::new(inference::InferenceEngine::new(model_manager.clone()).await?);
        let adaptation_engine = Arc::new(adaptation::AdaptationEngine::new(knowledge_base.clone()).await?);
        
        Ok(Self {
            config: config.clone(),
            model_manager,
            trainer,
            inference_engine,
            adaptation_engine,
            knowledge_base,
            learning_history: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Predict future needs based on context
    pub async fn predict_future_needs(&self, context: &Context) -> Result<Vec<Prediction>> {
        // Use inference engine to make predictions
        let predictions = self.inference_engine.predict_needs(context).await?;
        
        // Filter by confidence threshold
        Ok(predictions.into_iter()
            .filter(|p| p.confidence >= 0.5)
            .collect())
    }
    
    /// Process cycle outcomes for learning
    pub async fn process_cycle_outcomes(&self, context: &Context) -> Result<()> {
        // Extract learning data from context
        let learning_data = self.extract_learning_data(context).await?;
        
        // Store in knowledge base
        self.knowledge_base.store_learning_data(&learning_data).await?;
        
        // Perform online learning if enabled
        if self.config.enable_online_learning {
            self.trainer.online_update(&learning_data).await?;
        }
        
        // Record learning event
        let mut history = self.learning_history.write().await;
        history.push(LearningEvent {
            timestamp: Utc::now(),
            event_type: LearningEventType::CycleOutcome,
            data: serde_json::to_value(&learning_data)?,
        });
        
        Ok(())
    }
    
    /// Learn from error
    pub async fn learn_from_error(&self, healing_result: HealingResult) -> Result<()> {
        // Create error learning data
        let error_data = ErrorLearningData {
            error_type: healing_result.error_type,
            healing_success: healing_result.success,
            actions_taken: healing_result.actions_taken,
            timestamp: Utc::now(),
        };
        
        // Update error patterns
        self.adaptation_engine.update_error_patterns(&error_data).await?;
        
        // Record learning event
        let mut history = self.learning_history.write().await;
        history.push(LearningEvent {
            timestamp: Utc::now(),
            event_type: LearningEventType::ErrorRecovery,
            data: serde_json::to_value(&error_data)?,
        });
        
        Ok(())
    }
    
    /// Process decision outcomes
    pub async fn process_decision_outcomes(
        &self,
        records: &[crate::core::decision_engine::DecisionRecord],
    ) -> Result<()> {
        // Analyze decision patterns
        let patterns = self.analyze_decision_patterns(records).await?;
        
        // Update models
        self.trainer.update_decision_models(&patterns).await?;
        
        // Adapt strategies
        self.adaptation_engine.adapt_strategies(&patterns).await?;
        
        Ok(())
    }
    
    /// Start the learning system
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting Learning System");
        
        // Start training loop
        self.start_training_loop().await?;
        
        // Start adaptation loop
        self.adaptation_engine.start().await?;
        
        Ok(())
    }
    
    /// Stop the learning system
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping Learning System");
        
        // Save models
        self.model_manager.save_all_models().await?;
        
        // Persist learning history
        let history = self.learning_history.read().await;
        self.knowledge_base.store_learning_history(&*history).await?;
        
        Ok(())
    }
    
    /// Extract learning data from context
    async fn extract_learning_data(&self, context: &Context) -> Result<LearningData> {
        Ok(LearningData {
            timestamp: context.timestamp,
            system_health: context.system_health,
            resource_utilization: context.resources.clone(),
            active_tasks: context.active_tasks.len(),
            anomaly_count: context.anomalies.len(),
            metrics: context.metrics.clone(),
            features: self.extract_features(context).await?,
        })
    }
    
    /// Extract features for ML
    async fn extract_features(&self, context: &Context) -> Result<Vec<f64>> {
        let mut features = vec![
            context.system_health,
            context.resources.cpu_usage / 100.0,
            context.resources.memory_usage / 100.0,
            context.active_tasks.len() as f64 / 1000.0,
            context.anomalies.len() as f64 / 10.0,
        ];
        
        // Add time-based features
        let hour = context.timestamp.hour() as f64 / 24.0;
        let day_of_week = context.timestamp.weekday().num_days_from_monday() as f64 / 7.0;
        features.push(hour);
        features.push(day_of_week);
        
        Ok(features)
    }
    
    /// Analyze decision patterns
    async fn analyze_decision_patterns(
        &self,
        records: &[crate::core::decision_engine::DecisionRecord],
    ) -> Result<DecisionPatterns> {
        let mut success_by_type = std::collections::HashMap::new();
        let mut failure_by_type = std::collections::HashMap::new();
        let mut avg_execution_time = std::collections::HashMap::new();
        
        for record in records {
            if let Some(outcome) = &record.outcome {
                let decision_type = record.decision.decision_type();
                
                if outcome.success {
                    *success_by_type.entry(decision_type).or_insert(0) += 1;
                } else {
                    *failure_by_type.entry(decision_type).or_insert(0) += 1;
                }
                
                let times = avg_execution_time.entry(decision_type).or_insert(vec![]);
                times.push(outcome.execution_time.as_secs_f64());
            }
        }
        
        Ok(DecisionPatterns {
            success_rates: success_by_type,
            failure_rates: failure_by_type,
            avg_execution_times: avg_execution_time.into_iter()
                .map(|(k, v)| (k, v.iter().sum::<f64>() / v.len() as f64))
                .collect(),
        })
    }
    
    /// Start training loop
    async fn start_training_loop(&self) -> Result<()> {
        let trainer = self.trainer.clone();
        let interval = std::time::Duration::from_secs(self.config.training_interval_hours * 3600);
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                if let Err(e) = trainer.run_training_cycle().await {
                    tracing::error!("Training cycle failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
}

/// Prediction from the learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub prediction_type: PredictionType,
    pub confidence: f64,
    pub predicted_value: serde_json::Value,
    pub time_horizon: chrono::Duration,
    pub created_at: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionType {
    ResourceNeed,
    TaskLoad,
    FailureProbability,
    OptimizationOpportunity,
    MaintenanceWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningData {
    timestamp: DateTime<Utc>,
    system_health: f64,
    resource_utilization: crate::core::context::ResourceUtilization,
    active_tasks: usize,
    anomaly_count: usize,
    metrics: std::collections::HashMap<String, crate::core::context::MetricValue>,
    features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorLearningData {
    error_type: crate::execution::ErrorType,
    healing_success: bool,
    actions_taken: Vec<String>,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningEvent {
    timestamp: DateTime<Utc>,
    event_type: LearningEventType,
    data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum LearningEventType {
    CycleOutcome,
    ErrorRecovery,
    ModelUpdate,
    StrategyAdaptation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionPatterns {
    success_rates: std::collections::HashMap<&'static str, u32>,
    failure_rates: std::collections::HashMap<&'static str, u32>,
    avg_execution_times: std::collections::HashMap<&'static str, f64>,
}