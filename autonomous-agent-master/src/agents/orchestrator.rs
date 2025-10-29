/// Agent orchestrator - manages and coordinates autonomous agents
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, instrument};

use crate::{
    execution::ExecutionEngine,
    knowledge::KnowledgeBase,
};

use super::{
    Task, TaskStatus, TaskResult, SpecializedAgent, AgentType,
    Capability, CoordinationProtocol,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub max_agents: usize,
    pub agent_timeout_seconds: u64,
    pub task_queue_size: usize,
    pub enable_auto_scaling: bool,
    pub coordination_protocol: CoordinationProtocol,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_agents: 100,
            agent_timeout_seconds: 3600,
            task_queue_size: 10000,
            enable_auto_scaling: true,
            coordination_protocol: CoordinationProtocol::Hierarchical,
        }
    }
}

/// Orchestrates multiple autonomous agents
pub struct AgentOrchestrator {
    config: AgentConfig,
    agents: Arc<DashMap<String, Arc<SpecializedAgent>>>,
    task_queue: Arc<RwLock<Vec<Task>>>,
    active_tasks: Arc<DashMap<String, Task>>,
    execution_engine: Arc<ExecutionEngine>,
    knowledge_base: Arc<KnowledgeBase>,
}

impl AgentOrchestrator {
    pub async fn new(
        config: &AgentConfig,
        execution_engine: Arc<ExecutionEngine>,
        knowledge_base: Arc<KnowledgeBase>,
    ) -> Result<Self> {
        info!("Initializing Agent Orchestrator");
        
        let orchestrator = Self {
            config: config.clone(),
            agents: Arc::new(DashMap::new()),
            task_queue: Arc::new(RwLock::new(Vec::with_capacity(config.task_queue_size))),
            active_tasks: Arc::new(DashMap::new()),
            execution_engine,
            knowledge_base,
        };
        
        // Initialize default agents
        orchestrator.initialize_agents().await?;
        
        Ok(orchestrator)
    }
    
    /// Initialize specialized agents
    async fn initialize_agents(&self) -> Result<()> {
        // Create different types of specialized agents
        let agent_types = vec![
            AgentType::Developer,
            AgentType::Tester,
            AgentType::Analyst,
            AgentType::Operator,
            AgentType::Learner,
            AgentType::Communicator,
        ];
        
        for agent_type in agent_types {
            for i in 0..self.get_agent_count_for_type(&agent_type) {
                let agent = Arc::new(
                    SpecializedAgent::new(
                        format!("{:?}-{}", agent_type, i),
                        agent_type.clone(),
                        self.execution_engine.clone(),
                        self.knowledge_base.clone(),
                    ).await?
                );
                
                self.agents.insert(agent.id().to_string(), agent);
            }
        }
        
        info!("Initialized {} agents", self.agents.len());
        Ok(())
    }
    
    /// Get recommended agent count for type
    fn get_agent_count_for_type(&self, agent_type: &AgentType) -> usize {
        match agent_type {
            AgentType::Developer => 5,
            AgentType::Tester => 3,
            AgentType::Analyst => 3,
            AgentType::Operator => 2,
            AgentType::Learner => 2,
            AgentType::Communicator => 1,
        }
    }
    
    /// Execute a task
    #[instrument(skip(self, task))]
    pub async fn execute_task(&self, mut task: Task) -> Result<TaskResult> {
        info!("Executing task: {}", task.name);
        
        // Add to active tasks
        self.active_tasks.insert(task.id.clone(), task.clone());
        
        // Find suitable agent
        let agent = self.find_suitable_agent(&task).await?;
        
        // Assign task to agent
        task.assigned_agent = Some(agent.id().to_string());
        task.status = TaskStatus::Assigned;
        
        // Execute task
        let result = agent.execute_task(task.clone()).await?;
        
        // Remove from active tasks
        self.active_tasks.remove(&task.id);
        
        // Store result in knowledge base
        self.knowledge_base.store_task_result(&result).await?;
        
        Ok(result)
    }
    
    /// Find suitable agent for task
    async fn find_suitable_agent(&self, task: &Task) -> Result<Arc<SpecializedAgent>> {
        // Find agents with required capabilities
        let suitable_agents: Vec<_> = self.agents.iter()
            .filter(|entry| {
                let agent = entry.value();
                agent.has_capabilities(&task.requirements) && !agent.is_busy()
            })
            .map(|entry| entry.value().clone())
            .collect();
        
        if suitable_agents.is_empty() {
            // Try to create new agent if auto-scaling enabled
            if self.config.enable_auto_scaling && self.agents.len() < self.config.max_agents {
                return self.create_agent_for_task(task).await;
            }
            
            return Err(anyhow::anyhow!("No suitable agent available for task"));
        }
        
        // Select best agent based on load and expertise
        let best_agent = suitable_agents.into_iter()
            .min_by_key(|agent| agent.get_load())
            .context("Failed to select agent")?;
        
        Ok(best_agent)
    }
    
    /// Create new agent for specific task
    async fn create_agent_for_task(&self, task: &Task) -> Result<Arc<SpecializedAgent>> {
        let agent_type = match task.task_type {
            super::TaskType::Development => AgentType::Developer,
            super::TaskType::Testing => AgentType::Tester,
            super::TaskType::Analysis => AgentType::Analyst,
            super::TaskType::Deployment => AgentType::Operator,
            super::TaskType::Learning => AgentType::Learner,
            _ => AgentType::Operator,
        };
        
        let agent_id = format!("{:?}-auto-{}", agent_type, uuid::Uuid::new_v4());
        let agent = Arc::new(
            SpecializedAgent::new(
                agent_id.clone(),
                agent_type,
                self.execution_engine.clone(),
                self.knowledge_base.clone(),
            ).await?
        );
        
        self.agents.insert(agent_id, agent.clone());
        
        info!("Created new agent for task: {}", agent.id());
        Ok(agent)
    }
    
    /// Get active tasks
    pub async fn get_active_tasks(&self) -> Result<Vec<super::TaskInfo>> {
        Ok(self.active_tasks.iter()
            .map(|entry| {
                let task = entry.value();
                super::TaskInfo {
                    id: task.id.clone(),
                    name: task.name.clone(),
                    status: match task.status {
                        TaskStatus::Pending => crate::core::context::TaskStatus::Pending,
                        TaskStatus::InProgress => crate::core::context::TaskStatus::Running,
                        TaskStatus::Completed => crate::core::context::TaskStatus::Completed,
                        TaskStatus::Failed => crate::core::context::TaskStatus::Failed,
                        TaskStatus::Cancelled => crate::core::context::TaskStatus::Cancelled,
                        _ => crate::core::context::TaskStatus::Pending,
                    },
                    started_at: task.created_at,
                    priority: task.priority.clone(),
                    assigned_agent: task.assigned_agent.clone(),
                }
            })
            .collect()
        )
    }
    
    /// Get pending task count
    pub async fn get_pending_task_count(&self) -> usize {
        let queue = self.task_queue.read().await;
        queue.len()
    }
    
    /// Add task to queue
    pub async fn queue_task(&self, task: Task) -> Result<()> {
        let mut queue = self.task_queue.write().await;
        
        if queue.len() >= self.config.task_queue_size {
            return Err(anyhow::anyhow!("Task queue is full"));
        }
        
        queue.push(task);
        
        // Sort by priority
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(())
    }
    
    /// Start the orchestrator
    pub async fn start(&self) -> Result<()> {
        info!("Starting Agent Orchestrator");
        
        // Start task processing loop
        self.start_task_processor().await?;
        
        // Start agent monitoring
        self.start_agent_monitor().await?;
        
        // Start coordination protocol
        self.start_coordination().await?;
        
        Ok(())
    }
    
    /// Stop the orchestrator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Agent Orchestrator");
        
        // Stop all agents
        for entry in self.agents.iter() {
            entry.value().stop().await?;
        }
        
        Ok(())
    }
    
    /// Start task processing loop
    async fn start_task_processor(&self) -> Result<()> {
        let task_queue = self.task_queue.clone();
        let orchestrator = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                // Process queued tasks
                let task = {
                    let mut queue = task_queue.write().await;
                    queue.pop()
                };
                
                if let Some(task) = task {
                    tokio::spawn({
                        let orchestrator = orchestrator.clone();
                        async move {
                            if let Err(e) = orchestrator.execute_task(task).await {
                                warn!("Task execution failed: {}", e);
                            }
                        }
                    });
                }
            }
        });
        
        Ok(())
    }
    
    /// Start agent monitoring
    async fn start_agent_monitor(&self) -> Result<()> {
        let agents = self.agents.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check agent health
                for entry in agents.iter() {
                    let agent = entry.value();
                    
                    if !agent.is_healthy().await {
                        warn!("Agent {} is unhealthy", agent.id());
                        // Could implement self-healing here
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start coordination protocol
    async fn start_coordination(&self) -> Result<()> {
        match self.config.coordination_protocol {
            CoordinationProtocol::Hierarchical => {
                // Implement hierarchical coordination
                info!("Using hierarchical coordination protocol");
            },
            CoordinationProtocol::Peer2Peer => {
                // Implement P2P coordination
                info!("Using peer-to-peer coordination protocol");
            },
            CoordinationProtocol::Consensus => {
                // Implement consensus-based coordination
                info!("Using consensus coordination protocol");
            },
        }
        
        Ok(())
    }
}

// Clone implementation for orchestrator
impl Clone for AgentOrchestrator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            agents: self.agents.clone(),
            task_queue: self.task_queue.clone(),
            active_tasks: self.active_tasks.clone(),
            execution_engine: self.execution_engine.clone(),
            knowledge_base: self.knowledge_base.clone(),
        }
    }
}