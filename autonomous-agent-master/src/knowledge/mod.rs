/// Knowledge base for storing and retrieving system knowledge
pub mod storage;
pub mod indexing;
pub mod retrieval;
pub mod learning_data;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::{
    agents::TaskResult,
    core::{context::Event, decision_engine::DecisionRecord},
};

#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeConfig {
    pub storage_backend: storage::StorageBackend,
    pub index_type: indexing::IndexType,
    pub retention_days: u32,
    pub enable_versioning: bool,
    pub enable_compression: bool,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            storage_backend: storage::StorageBackend::Postgres,
            index_type: indexing::IndexType::Vector,
            retention_days: 90,
            enable_versioning: true,
            enable_compression: true,
        }
    }
}

/// Central knowledge base for the system
pub struct KnowledgeBase {
    config: KnowledgeConfig,
    storage: Arc<storage::StorageEngine>,
    indexer: Arc<indexing::Indexer>,
    retriever: Arc<retrieval::Retriever>,
    cache: Arc<RwLock<KnowledgeCache>>,
}

impl KnowledgeBase {
    pub async fn new(config: &KnowledgeConfig) -> Result<Self> {
        tracing::info!("Initializing Knowledge Base");
        
        // Initialize storage
        let storage = Arc::new(storage::StorageEngine::new(&config.storage_backend).await?);
        
        // Initialize indexer
        let indexer = Arc::new(indexing::Indexer::new(&config.index_type, storage.clone()).await?);
        
        // Initialize retriever
        let retriever = Arc::new(retrieval::Retriever::new(storage.clone(), indexer.clone()).await?);
        
        Ok(Self {
            config: config.clone(),
            storage,
            indexer,
            retriever,
            cache: Arc::new(RwLock::new(KnowledgeCache::new())),
        })
    }
    
    /// Store task result
    pub async fn store_task_result(&self, result: &TaskResult) -> Result<()> {
        // Store in database
        self.storage.store_task_result(result).await?;
        
        // Update index
        self.indexer.index_task_result(result).await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.add_task_result(result.clone());
        
        Ok(())
    }
    
    /// Get recent events
    pub async fn get_recent_events(&self, limit: usize) -> Result<Vec<Event>> {
        // Check cache first
        let cache = self.cache.read().await;
        if let Some(events) = cache.get_recent_events(limit) {
            return Ok(events);
        }
        
        // Fallback to storage
        self.storage.get_recent_events(limit).await
    }
    
    /// Store decision history
    pub async fn store_decision_history(&self, history: &[DecisionRecord]) -> Result<()> {
        self.storage.store_decision_history(history).await
    }
    
    /// Get success rate for decision type
    pub async fn get_success_rate_for_decision_type(&self, decision_type: &str) -> Result<f64> {
        self.storage.get_decision_success_rate(decision_type).await
    }
    
    /// Get active predictions
    pub async fn get_active_predictions(&self) -> Result<Vec<crate::core::context::Prediction>> {
        self.storage.get_active_predictions().await
    }
    
    /// Update from cycle
    pub async fn update_from_cycle(&self, context: &crate::core::Context) -> Result<()> {
        // Store context snapshot
        self.storage.store_context_snapshot(context).await?;
        
        // Extract and store events
        for event in &context.recent_events {
            self.storage.store_event(event).await?;
        }
        
        // Update metrics
        for (name, value) in &context.metrics {
            self.storage.store_metric(name, value).await?;
        }
        
        Ok(())
    }
    
    /// Search knowledge base
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeItem>> {
        self.retriever.search(query, limit).await
    }
    
    /// Get insights for a topic
    pub async fn get_insights(&self, topic: &str) -> Result<Vec<Insight>> {
        self.retriever.get_insights_for_topic(topic).await
    }
    
    /// Persist all data
    pub async fn persist(&self) -> Result<()> {
        tracing::info!("Persisting knowledge base");
        
        // Flush cache to storage
        let cache = self.cache.read().await;
        self.storage.flush_cache(&cache).await?;
        
        // Optimize indices
        self.indexer.optimize().await?;
        
        Ok(())
    }
}

/// In-memory cache for frequently accessed data
struct KnowledgeCache {
    recent_events: Vec<Event>,
    task_results: Vec<TaskResult>,
    metrics: std::collections::HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    max_cache_size: usize,
}

impl KnowledgeCache {
    fn new() -> Self {
        Self {
            recent_events: Vec::new(),
            task_results: Vec::new(),
            metrics: std::collections::HashMap::new(),
            max_cache_size: 10000,
        }
    }
    
    fn add_task_result(&mut self, result: TaskResult) {
        self.task_results.push(result);
        
        // Maintain cache size
        if self.task_results.len() > self.max_cache_size {
            self.task_results.drain(0..1000);
        }
    }
    
    fn get_recent_events(&self, limit: usize) -> Option<Vec<Event>> {
        if self.recent_events.is_empty() {
            None
        } else {
            Some(
                self.recent_events
                    .iter()
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect()
            )
        }
    }
}

/// A knowledge item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub item_type: KnowledgeType,
    pub title: String,
    pub content: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeType {
    Documentation,
    Code,
    Configuration,
    Pattern,
    Issue,
    Solution,
    Metric,
    Event,
}

/// An insight derived from knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
    pub recommendations: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    Trend,
    Anomaly,
    Optimization,
    Risk,
    Opportunity,
}