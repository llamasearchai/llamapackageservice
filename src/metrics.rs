use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Collects and tracks application metrics
pub struct Metrics {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    timers: Arc<RwLock<HashMap<String, Duration>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
}

impl Metrics {
    /// Creates a new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            timers: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increments a counter metric by 1
    pub async fn increment(&self, key: &str) {
        let mut counters = self.counters.write().await;
        *counters.entry(key.to_string()).or_insert(0) += 1;
        debug!("Incremented counter {}: {}", key, counters[key]);
    }

    /// Decrements a counter metric by 1
    pub async fn decrement(&self, key: &str) {
        let mut counters = self.counters.write().await;
        let value = counters.entry(key.to_string()).or_insert(0);
        if *value > 0 {
            *value -= 1;
        }
        debug!("Decremented counter {}: {}", key, counters[key]);
    }

    /// Records a timing metric in milliseconds
    pub async fn record_time(&self, key: &str, duration: Duration) {
        let mut timers = self.timers.write().await;
        timers.insert(key.to_string(), duration);
        debug!("Recorded timer {}: {:?}", key, duration);
    }

    /// Sets a gauge metric to the specified value
    pub async fn set_gauge(&self, key: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(key.to_string(), value);
        debug!("Set gauge {}: {}", key, value);
    }

    /// Gets the current value of a counter metric
    pub async fn get_counter(&self, key: &str) -> Option<u64> {
        let counters = self.counters.read().await;
        counters.get(key).cloned()
    }

    /// Gets the current value of a timer metric
    pub async fn get_timer(&self, key: &str) -> Option<Duration> {
        let timers = self.timers.read().await;
        timers.get(key).cloned()
    }

    /// Gets the current value of a gauge metric
    pub async fn get_gauge(&self, key: &str) -> Option<f64> {
        let gauges = self.gauges.read().await;
        gauges.get(key).cloned()
    }

    /// Generates a report of all collected metrics
    pub async fn report(&self) {
        info!("=== Metrics Report ===");
        
        // Report counters
        let counters = self.counters.read().await;
        if !counters.is_empty() {
            info!("Counters:");
            for (k, v) in counters.iter() {
                info!("  {}: {}", k, v);
            }
        }
        
        // Report timers
        let timers = self.timers.read().await;
        if !timers.is_empty() {
            info!("Timers:");
            for (k, v) in timers.iter() {
                info!("  {}: {:?}", k, v);
            }
        }
        
        // Report gauges
        let gauges = self.gauges.read().await;
        if !gauges.is_empty() {
            info!("Gauges:");
            for (k, v) in gauges.iter() {
                info!("  {}: {}", k, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_counters() {
        let metrics = Metrics::new();
        
        metrics.increment("test_counter").await;
        metrics.increment("test_counter").await;
        
        let value = metrics.get_counter("test_counter").await.unwrap();
        assert_eq!(value, 2);
        
        metrics.decrement("test_counter").await;
        let value = metrics.get_counter("test_counter").await.unwrap();
        assert_eq!(value, 1);
    }
    
    #[tokio::test]
    async fn test_timers() {
        let metrics = Metrics::new();
        
        let duration = Duration::from_secs(5);
        metrics.record_time("test_timer", duration).await;
        
        let value = metrics.get_timer("test_timer").await.unwrap();
        assert_eq!(value, duration);
    }
    
    #[tokio::test]
    async fn test_gauges() {
        let metrics = Metrics::new();
        
        metrics.set_gauge("test_gauge", 42.5).await;
        
        let value = metrics.get_gauge("test_gauge").await.unwrap();
        assert_eq!(value, 42.5);
    }
}