/// Real-time monitoring and alerting system
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use chrono::{DateTime, Utc, Duration};
use std::collections::VecDeque;

#[derive(Debug, Clone, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_retention_hours: u64,
    pub alert_cooldown_minutes: u64,
    pub health_check_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_retention_hours: 24,
            alert_cooldown_minutes: 15,
            health_check_interval_seconds: 60,
        }
    }
}

pub struct SystemMonitor {
    config: MonitoringConfig,
    metrics: Arc<RwLock<MetricsStore>>,
    alerts: Arc<DashMap<String, Alert>>,
    health_checks: Arc<DashMap<String, HealthCheck>>,
}

impl SystemMonitor {
    pub async fn new(config: MonitoringConfig) -> Result<Self> {
        Ok(Self {
            config,
            metrics: Arc::new(RwLock::new(MetricsStore::new())),
            alerts: Arc::new(DashMap::new()),
            health_checks: Arc::new(DashMap::new()),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        // Start metrics collection
        let metrics_handle = self.start_metrics_collection();
        
        // Start health checks
        let health_handle = self.start_health_checks();
        
        // Start alert processing
        let alert_handle = self.start_alert_processing();
        
        tokio::try_join!(metrics_handle, health_handle, alert_handle)?;
        
        Ok(())
    }
    
    async fn start_metrics_collection(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        let interval = Duration::seconds(10);
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval.to_std().unwrap());
            
            loop {
                ticker.tick().await;
                
                // Collect system metrics
                let cpu_usage = get_cpu_usage().await;
                let memory_usage = get_memory_usage().await;
                let disk_usage = get_disk_usage().await;
                
                let mut store = metrics.write().await;
                store.record_metric("system.cpu", cpu_usage);
                store.record_metric("system.memory", memory_usage);
                store.record_metric("system.disk", disk_usage);
                
                // Clean old metrics
                store.clean_old_metrics(Duration::hours(24));
            }
        });
        
        Ok(())
    }
    
    async fn start_health_checks(&self) -> Result<()> {
        let health_checks = self.health_checks.clone();
        let alerts = self.alerts.clone();
        let interval = Duration::seconds(self.config.health_check_interval_seconds as i64);
        
        // Register default health checks
        self.register_health_check(HealthCheck {
            id: "github_api".to_string(),
            name: "GitHub API".to_string(),
            check_fn: Box::new(check_github_health),
            timeout_seconds: 30,
        }).await?;
        
        self.register_health_check(HealthCheck {
            id: "operate_bridge".to_string(),
            name: "Operate Python Bridge".to_string(),
            check_fn: Box::new(check_operate_health),
            timeout_seconds: 10,
        }).await?;
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval.to_std().unwrap());
            
            loop {
                ticker.tick().await;
                
                for entry in health_checks.iter() {
                    let check = entry.value();
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(check.timeout_seconds),
                        (check.check_fn)()
                    ).await;
                    
                    match result {
                        Ok(Ok(status)) => {
                            if !status.healthy {
                                alerts.insert(
                                    format!("health_{}", check.id),
                                    Alert {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        level: AlertLevel::Warning,
                                        source: format!("health_check.{}", check.id),
                                        message: format!("{} is unhealthy: {}", check.name, status.message),
                                        timestamp: Utc::now(),
                                        metadata: serde_json::json!({
                                            "check_id": check.id,
                                            "details": status.details,
                                        }),
                                    }
                                );
                            }
                        },
                        Ok(Err(e)) => {
                            alerts.insert(
                                format!("health_{}", check.id),
                                Alert {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    level: AlertLevel::Error,
                                    source: format!("health_check.{}", check.id),
                                    message: format!("{} check failed: {}", check.name, e),
                                    timestamp: Utc::now(),
                                    metadata: serde_json::json!({"check_id": check.id}),
                                }
                            );
                        },
                        Err(_) => {
                            alerts.insert(
                                format!("health_{}", check.id),
                                Alert {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    level: AlertLevel::Critical,
                                    source: format!("health_check.{}", check.id),
                                    message: format!("{} check timed out", check.name),
                                    timestamp: Utc::now(),
                                    metadata: serde_json::json!({"check_id": check.id}),
                                }
                            );
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn start_alert_processing(&self) -> Result<()> {
        let alerts = self.alerts.clone();
        let cooldown = Duration::minutes(self.config.alert_cooldown_minutes as i64);
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
            let mut last_alerts: DashMap<String, DateTime<Utc>> = DashMap::new();
            
            loop {
                ticker.tick().await;
                
                // Process alerts
                let mut to_notify = Vec::new();
                
                for entry in alerts.iter() {
                    let alert = entry.value();
                    let key = format!("{}:{}", alert.source, alert.level.to_string());
                    
                    // Check cooldown
                    if let Some(last_time) = last_alerts.get(&key) {
                        if Utc::now() - *last_time < cooldown {
                            continue;
                        }
                    }
                    
                    to_notify.push(alert.clone());
                    last_alerts.insert(key, Utc::now());
                }
                
                // Send notifications
                for alert in to_notify {
                    if let Err(e) = notify_alert(&alert).await {
                        tracing::error!("Failed to send alert notification: {}", e);
                    }
                }
                
                // Clean old alerts
                alerts.retain(|_, alert| {
                    Utc::now() - alert.timestamp < Duration::hours(1)
                });
            }
        });
        
        Ok(())
    }
    
    pub async fn register_health_check(&self, check: HealthCheck) -> Result<()> {
        self.health_checks.insert(check.id.clone(), check);
        Ok(())
    }
    
    pub async fn record_metric(&self, name: &str, value: f64) {
        let mut store = self.metrics.write().await;
        store.record_metric(name, value);
    }
    
    pub async fn create_alert(&self, alert: Alert) {
        self.alerts.insert(alert.id.clone(), alert);
    }
    
    pub async fn get_metrics(&self, name: &str, duration: Duration) -> Vec<MetricPoint> {
        let store = self.metrics.read().await;
        store.get_metrics(name, duration)
    }
    
    pub async fn get_system_status(&self) -> SystemStatus {
        let metrics = self.metrics.read().await;
        let active_alerts: Vec<Alert> = self.alerts.iter()
            .map(|e| e.value().clone())
            .collect();
        
        let health_status: Vec<(String, bool)> = self.health_checks.iter()
            .map(|e| {
                let check = e.value();
                let healthy = !self.alerts.contains_key(&format!("health_{}", check.id));
                (check.name.clone(), healthy)
            })
            .collect();
        
        SystemStatus {
            healthy: active_alerts.iter().all(|a| a.level != AlertLevel::Critical),
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            active_alerts: active_alerts.len(),
            critical_alerts: active_alerts.iter()
                .filter(|a| a.level == AlertLevel::Critical)
                .count(),
            health_checks: health_status,
            recent_metrics: metrics.get_recent_summary(),
        }
    }
}

struct MetricsStore {
    metrics: DashMap<String, VecDeque<MetricPoint>>,
}

impl MetricsStore {
    fn new() -> Self {
        Self {
            metrics: DashMap::new(),
        }
    }
    
    fn record_metric(&mut self, name: &str, value: f64) {
        let point = MetricPoint {
            timestamp: Utc::now(),
            value,
        };
        
        self.metrics.entry(name.to_string())
            .or_insert_with(VecDeque::new)
            .push_back(point);
    }
    
    fn get_metrics(&self, name: &str, duration: Duration) -> Vec<MetricPoint> {
        let cutoff = Utc::now() - duration;
        
        self.metrics.get(name)
            .map(|deque| {
                deque.iter()
                    .filter(|p| p.timestamp > cutoff)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    fn clean_old_metrics(&mut self, retention: Duration) {
        let cutoff = Utc::now() - retention;
        
        for mut entry in self.metrics.iter_mut() {
            entry.value_mut().retain(|p| p.timestamp > cutoff);
        }
    }
    
    fn get_recent_summary(&self) -> serde_json::Value {
        let mut summary = serde_json::Map::new();
        
        for entry in self.metrics.iter() {
            if let Some(last) = entry.value().back() {
                summary.insert(entry.key().clone(), serde_json::json!({
                    "value": last.value,
                    "timestamp": last.timestamp.to_rfc3339(),
                }));
            }
        }
        
        serde_json::Value::Object(summary)
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub id: String,
    pub name: String,
    pub check_fn: Arc<dyn Fn() -> futures::future::BoxFuture<'static, Result<HealthStatus>> + Send + Sync>,
    pub timeout_seconds: u64,
}

// Manual implementation for boxing the check function
impl HealthCheck {
    pub fn new<F, Fut>(id: String, name: String, check_fn: F, timeout_seconds: u64) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: futures::Future<Output = Result<HealthStatus>> + Send + 'static,
    {
        Self {
            id,
            name,
            check_fn: Arc::new(move || Box::pin(check_fn())),
            timeout_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl ToString for AlertLevel {
    fn to_string(&self) -> String {
        match self {
            AlertLevel::Info => "info",
            AlertLevel::Warning => "warning",
            AlertLevel::Error => "error",
            AlertLevel::Critical => "critical",
        }.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub healthy: bool,
    pub uptime: u64,
    pub active_alerts: usize,
    pub critical_alerts: usize,
    pub health_checks: Vec<(String, bool)>,
    pub recent_metrics: serde_json::Value,
}

// Helper functions
async fn get_cpu_usage() -> f64 {
    // Implementation would use system metrics
    rand::random::<f64>() * 100.0
}

async fn get_memory_usage() -> f64 {
    // Implementation would use system metrics
    rand::random::<f64>() * 100.0
}

async fn get_disk_usage() -> f64 {
    // Implementation would use system metrics
    rand::random::<f64>() * 100.0
}

async fn check_github_health() -> Result<HealthStatus> {
    // Implementation would check GitHub API
    Ok(HealthStatus {
        healthy: true,
        message: "GitHub API is responsive".to_string(),
        details: None,
    })
}

async fn check_operate_health() -> Result<HealthStatus> {
    // Implementation would check Python bridge
    Ok(HealthStatus {
        healthy: true,
        message: "Operate bridge is functional".to_string(),
        details: None,
    })
}

async fn notify_alert(alert: &Alert) -> Result<()> {
    // Implementation would send notifications (email, Slack, etc.)
    tracing::warn!("Alert: {} - {}", alert.level.to_string(), alert.message);
    Ok(())
}

// Add rand to dependencies for demo
use rand;