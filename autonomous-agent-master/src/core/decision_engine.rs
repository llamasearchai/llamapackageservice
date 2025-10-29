/// Autonomous decision engine - the brain of the system
use anyhow::{Result, Context as AnyhowContext};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, instrument};

use crate::{
    agents::AgentOrchestrator,
    knowledge::KnowledgeBase,
    learning::LearningSystem,
};

use super::{Context, Decision, Priority, Rule, RuleEngine, Strategy, StrategyType};

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionConfig {
    pub max_concurrent_decisions: usize,
    pub decision_timeout_seconds: u64,
    pub confidence_threshold: f64,
    pub enable_predictive_decisions: bool,
    pub enable_autonomous_learning: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_decisions: 10,
            decision_timeout_seconds: 300,
            confidence_threshold: 0.7,
            enable_predictive_decisions: true,
            enable_autonomous_learning: true,
        }
    }
}

/// The autonomous decision engine
pub struct DecisionEngine {
    config: DecisionConfig,
    rule_engine: Arc<RuleEngine>,
    strategies: Arc<DashMap<StrategyType, Box<dyn Strategy>>>,
    agent_orchestrator: Arc<AgentOrchestrator>,
    knowledge_base: Arc<KnowledgeBase>,
    learning_system: Arc<LearningSystem>,
    decision_history: Arc<RwLock<Vec<DecisionRecord>>>,
}

impl DecisionEngine {
    pub async fn new(
        config: &DecisionConfig,
        agent_orchestrator: Arc<AgentOrchestrator>,
        knowledge_base: Arc<KnowledgeBase>,
        learning_system: Arc<LearningSystem>,
    ) -> Result<Self> {
        info!("Initializing Decision Engine");
        
        // Initialize rule engine
        let rule_engine = Arc::new(RuleEngine::new().await?);
        
        // Initialize strategies
        let strategies = Arc::new(DashMap::new());
        Self::load_strategies(&strategies).await?;
        
        Ok(Self {
            config: config.clone(),
            rule_engine,
            strategies,
            agent_orchestrator,
            knowledge_base,
            learning_system,
            decision_history: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Make decisions based on the current context
    #[instrument(skip(self, context))]
    pub async fn make_decisions(&self, context: &Context) -> Result<Vec<Decision>> {
        info!("Making decisions based on context");
        
        let mut decisions = Vec::new();
        
        // 1. Apply rules for immediate decisions
        let rule_decisions = self.apply_rules(context).await?;
        decisions.extend(rule_decisions);
        
        // 2. Apply strategies for planned decisions
        let strategy_decisions = self.apply_strategies(context).await?;
        decisions.extend(strategy_decisions);
        
        // 3. Predictive decisions based on patterns
        if self.config.enable_predictive_decisions {
            let predictive_decisions = self.make_predictive_decisions(context).await?;
            decisions.extend(predictive_decisions);
        }
        
        // 4. Prioritize and filter decisions
        let final_decisions = self.prioritize_decisions(decisions, context).await?;
        
        // 5. Record decisions for learning
        self.record_decisions(&final_decisions).await?;
        
        Ok(final_decisions)
    }
    
    /// Apply rules to generate immediate decisions
    async fn apply_rules(&self, context: &Context) -> Result<Vec<Decision>> {
        let mut decisions = Vec::new();
        
        // Get applicable rules
        let rules = self.rule_engine.get_applicable_rules(context).await?;
        
        for rule in rules {
            if let Some(decision) = self.evaluate_rule(&rule, context).await? {
                decisions.push(decision);
            }
        }
        
        Ok(decisions)
    }
    
    /// Apply strategies for planned decisions
    async fn apply_strategies(&self, context: &Context) -> Result<Vec<Decision>> {
        let mut decisions = Vec::new();
        
        // Determine which strategies to apply
        let applicable_strategies = self.determine_applicable_strategies(context).await?;
        
        for strategy_type in applicable_strategies {
            if let Some(strategy) = self.strategies.get(&strategy_type) {
                let strategy_decisions = strategy.generate_decisions(context).await?;
                decisions.extend(strategy_decisions);
            }
        }
        
        Ok(decisions)
    }
    
    /// Make predictive decisions based on learned patterns
    async fn make_predictive_decisions(&self, context: &Context) -> Result<Vec<Decision>> {
        let mut decisions = Vec::new();
        
        // Query learning system for predictions
        let predictions = self.learning_system.predict_future_needs(context).await?;
        
        for prediction in predictions {
            if prediction.confidence >= self.config.confidence_threshold {
                if let Some(decision) = self.create_decision_from_prediction(prediction).await? {
                    decisions.push(decision);
                }
            }
        }
        
        Ok(decisions)
    }
    
    /// Prioritize and filter decisions
    async fn prioritize_decisions(
        &self,
        mut decisions: Vec<Decision>,
        context: &Context,
    ) -> Result<Vec<Decision>> {
        // Remove duplicates
        decisions.dedup_by_key(|d| d.id());
        
        // Score decisions based on context
        for decision in &mut decisions {
            decision.set_score(self.score_decision(decision, context).await?);
        }
        
        // Sort by priority and score
        decisions.sort_by(|a, b| {
            match a.priority().cmp(&b.priority()) {
                std::cmp::Ordering::Equal => {
                    b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal)
                },
                other => other,
            }
        });
        
        // Take only top decisions based on capacity
        decisions.truncate(self.config.max_concurrent_decisions);
        
        Ok(decisions)
    }
    
    /// Evaluate a rule against context
    async fn evaluate_rule(&self, rule: &Rule, context: &Context) -> Result<Option<Decision>> {
        if rule.evaluate(context).await? {
            Ok(Some(rule.generate_decision(context).await?))
        } else {
            Ok(None)
        }
    }
    
    /// Determine which strategies are applicable
    async fn determine_applicable_strategies(&self, context: &Context) -> Result<Vec<StrategyType>> {
        let mut strategies = Vec::new();
        
        // Check system health
        if context.system_health < 0.8 {
            strategies.push(StrategyType::Recovery);
        }
        
        // Check for optimization opportunities
        if context.has_optimization_opportunities() {
            strategies.push(StrategyType::Optimization);
        }
        
        // Check for scaling needs
        if context.requires_scaling() {
            strategies.push(StrategyType::Scaling);
        }
        
        // Always apply maintenance strategy
        strategies.push(StrategyType::Maintenance);
        
        Ok(strategies)
    }
    
    /// Create decision from prediction
    async fn create_decision_from_prediction(
        &self,
        prediction: learning::Prediction,
    ) -> Result<Option<Decision>> {
        match prediction.prediction_type {
            learning::PredictionType::ResourceNeed => {
                Ok(Some(Decision::ScaleResources(
                    prediction.data.into()
                )))
            },
            learning::PredictionType::MaintenanceWindow => {
                Ok(Some(Decision::TriggerWorkflow(
                    execution::Workflow {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: "preventive_maintenance".to_string(),
                        trigger: execution::WorkflowTrigger::Scheduled,
                        steps: vec![],
                    }
                )))
            },
            _ => Ok(None),
        }
    }
    
    /// Score a decision based on context
    async fn score_decision(&self, decision: &Decision, context: &Context) -> Result<f64> {
        let mut score = 0.5; // Base score
        
        // Factor in urgency
        if decision.is_urgent() {
            score += 0.2;
        }
        
        // Factor in impact
        score += decision.estimated_impact() * 0.3;
        
        // Factor in resource availability
        if context.has_resources_for(decision) {
            score += 0.1;
        }
        
        // Factor in success probability
        let success_prob = self.estimate_success_probability(decision, context).await?;
        score *= success_prob;
        
        Ok(score.min(1.0))
    }
    
    /// Estimate success probability
    async fn estimate_success_probability(
        &self,
        decision: &Decision,
        context: &Context,
    ) -> Result<f64> {
        // Query historical success rates
        let historical_rate = self.knowledge_base
            .get_success_rate_for_decision_type(decision.decision_type())
            .await?;
        
        // Adjust based on current context
        let context_adjustment = if context.system_health > 0.9 {
            0.1
        } else if context.system_health < 0.5 {
            -0.2
        } else {
            0.0
        };
        
        Ok((historical_rate + context_adjustment).max(0.0).min(1.0))
    }
    
    /// Record decisions for learning
    async fn record_decisions(&self, decisions: &[Decision]) -> Result<()> {
        let mut history = self.decision_history.write().await;
        
        for decision in decisions {
            history.push(DecisionRecord {
                decision: decision.clone(),
                timestamp: chrono::Utc::now(),
                context_hash: "".to_string(), // Simplified
                outcome: None,
            });
        }
        
        // Keep only recent history
        if history.len() > 10000 {
            history.drain(0..5000);
        }
        
        Ok(())
    }
    
    /// Load all strategies
    async fn load_strategies(strategies: &DashMap<StrategyType, Box<dyn Strategy>>) -> Result<()> {
        // Load built-in strategies
        strategies.insert(
            StrategyType::Optimization,
            Box::new(strategies::OptimizationStrategy::new())
        );
        
        strategies.insert(
            StrategyType::Scaling,
            Box::new(strategies::ScalingStrategy::new())
        );
        
        strategies.insert(
            StrategyType::Recovery,
            Box::new(strategies::RecoveryStrategy::new())
        );
        
        strategies.insert(
            StrategyType::Maintenance,
            Box::new(strategies::MaintenanceStrategy::new())
        );
        
        Ok(())
    }
    
    /// Start the decision engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting Decision Engine");
        
        // Start rule engine
        self.rule_engine.start().await?;
        
        // Start background tasks
        self.start_decision_monitoring().await?;
        
        Ok(())
    }
    
    /// Stop the decision engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Decision Engine");
        
        // Persist decision history
        let history = self.decision_history.read().await;
        self.knowledge_base.store_decision_history(&*history).await?;
        
        Ok(())
    }
    
    /// Start monitoring decision outcomes
    async fn start_decision_monitoring(&self) -> Result<()> {
        let history = self.decision_history.clone();
        let learning_system = self.learning_system.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Check for completed decisions and update outcomes
                let mut write_guard = history.write().await;
                for record in write_guard.iter_mut() {
                    if record.outcome.is_none() {
                        // Check if decision has completed
                        // This would query the execution system
                    }
                }
                
                // Send outcomes to learning system
                let completed_records: Vec<_> = write_guard.iter()
                    .filter(|r| r.outcome.is_some())
                    .cloned()
                    .collect();
                
                drop(write_guard);
                
                if !completed_records.is_empty() {
                    if let Err(e) = learning_system.process_decision_outcomes(&completed_records).await {
                        warn!("Failed to process decision outcomes: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionRecord {
    decision: Decision,
    timestamp: chrono::DateTime<chrono::Utc>,
    context_hash: String,
    outcome: Option<DecisionOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionOutcome {
    success: bool,
    impact_score: f64,
    execution_time: std::time::Duration,
    error: Option<String>,
}

// Re-exports for convenience
use crate::{execution, learning};