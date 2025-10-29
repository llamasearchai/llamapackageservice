/// External integrations manager
pub mod github;
pub mod cloud;
pub mod messaging;
pub mod monitoring;
pub mod ai_providers;

use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationConfig {
    pub github: github::GitHubConfig,
    pub cloud: cloud::CloudConfig,
    pub messaging: messaging::MessagingConfig,
    pub monitoring: monitoring::MonitoringConfig,
    pub ai_providers: ai_providers::AIConfig,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            github: Default::default(),
            cloud: Default::default(),
            messaging: Default::default(),
            monitoring: Default::default(),
            ai_providers: Default::default(),
        }
    }
}

/// Manages all external integrations
pub struct IntegrationManager {
    config: IntegrationConfig,
    integrations: Arc<DashMap<String, Box<dyn Integration>>>,
    github: Arc<github::GitHubIntegration>,
    cloud: Arc<cloud::CloudIntegration>,
    messaging: Arc<messaging::MessagingIntegration>,
    monitoring: Arc<monitoring::MonitoringIntegration>,
    ai_providers: Arc<ai_providers::AIProviders>,
}

impl IntegrationManager {
    pub async fn new(config: &IntegrationConfig) -> Result<Self> {
        tracing::info!("Initializing Integration Manager");
        
        // Initialize individual integrations
        let github = Arc::new(github::GitHubIntegration::new(&config.github).await?);
        let cloud = Arc::new(cloud::CloudIntegration::new(&config.cloud).await?);
        let messaging = Arc::new(messaging::MessagingIntegration::new(&config.messaging).await?);
        let monitoring = Arc::new(monitoring::MonitoringIntegration::new(&config.monitoring).await?);
        let ai_providers = Arc::new(ai_providers::AIProviders::new(&config.ai_providers).await?);
        
        let manager = Self {
            config: config.clone(),
            integrations: Arc::new(DashMap::new()),
            github,
            cloud,
            messaging,
            monitoring,
            ai_providers,
        };
        
        // Register all integrations
        manager.register_integrations().await?;
        
        Ok(manager)
    }
    
    /// Register all integrations
    async fn register_integrations(&self) -> Result<()> {
        // This would register dynamic integrations
        Ok(())
    }
    
    /// Get status of all integrations
    pub async fn get_all_status(&self) -> Result<std::collections::HashMap<String, crate::core::context::IntegrationStatus>> {
        let mut status_map = std::collections::HashMap::new();
        
        // Get GitHub status
        let github_status = self.github.get_status().await?;
        status_map.insert("github".to_string(), github_status);
        
        // Get cloud status
        let cloud_status = self.cloud.get_status().await?;
        status_map.insert("cloud".to_string(), cloud_status);
        
        // Get messaging status
        let messaging_status = self.messaging.get_status().await?;
        status_map.insert("messaging".to_string(), messaging_status);
        
        // Get monitoring status
        let monitoring_status = self.monitoring.get_status().await?;
        status_map.insert("monitoring".to_string(), monitoring_status);
        
        // Get AI providers status
        let ai_status = self.ai_providers.get_status().await?;
        status_map.insert("ai_providers".to_string(), ai_status);
        
        Ok(status_map)
    }
    
    /// Get GitHub integration
    pub fn github(&self) -> &github::GitHubIntegration {
        &self.github
    }
    
    /// Get cloud integration
    pub fn cloud(&self) -> &cloud::CloudIntegration {
        &self.cloud
    }
    
    /// Get messaging integration
    pub fn messaging(&self) -> &messaging::MessagingIntegration {
        &self.messaging
    }
    
    /// Get monitoring integration
    pub fn monitoring(&self) -> &monitoring::MonitoringIntegration {
        &self.monitoring
    }
    
    /// Get AI providers
    pub fn ai_providers(&self) -> &ai_providers::AIProviders {
        &self.ai_providers
    }
    
    /// Start all integrations
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting all integrations");
        
        tokio::try_join!(
            self.github.start(),
            self.cloud.start(),
            self.messaging.start(),
            self.monitoring.start(),
            self.ai_providers.start(),
        )?;
        
        Ok(())
    }
    
    /// Stop all integrations
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping all integrations");
        
        tokio::try_join!(
            self.github.stop(),
            self.cloud.stop(),
            self.messaging.stop(),
            self.monitoring.stop(),
            self.ai_providers.stop(),
        )?;
        
        Ok(())
    }
}

/// Base trait for all integrations
#[async_trait]
pub trait Integration: Send + Sync {
    /// Get integration name
    fn name(&self) -> &str;
    
    /// Get integration status
    async fn get_status(&self) -> Result<crate::core::context::IntegrationStatus>;
    
    /// Start the integration
    async fn start(&self) -> Result<()>;
    
    /// Stop the integration
    async fn stop(&self) -> Result<()>;
    
    /// Health check
    async fn health_check(&self) -> Result<bool>;
}