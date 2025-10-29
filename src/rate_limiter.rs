use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

/// Limits the rate of operations to prevent API rate limit issues
pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
    state: Arc<Mutex<HashMap<String, RateLimitState>>>,
}

#[derive(Clone)]
struct RateLimit {
    requests: usize,
    window: Duration,
}

struct RateLimitState {
    requests: Vec<Instant>,
}

impl RateLimiter {
    /// Creates a new rate limiter with default settings
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        
        // Add default rate limits
        limits.insert(
            "github".to_string(),
            RateLimit {
                requests: 60,
                window: Duration::from_secs(60),
            },
        );
        
        limits.insert(
            "pypi".to_string(),
            RateLimit {
                requests: 100,
                window: Duration::from_secs(60),
            },
        );
        
        Self {
            limits,
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Acquires a permit for the specified key, waiting if necessary
    pub async fn acquire(&self, key: &str) {
        let limit = self.limits.get(key).cloned().unwrap_or(RateLimit {
            requests: 10,
            window: Duration::from_secs(60),
        });
        
        loop {
            let mut state = self.state.lock().await;
            let entry = state.entry(key.to_string()).or_insert_with(|| RateLimitState {
                requests: Vec::new(),
            });
            
            // Remove expired timestamps
            let now = Instant::now();
            entry.requests.retain(|&t| now.duration_since(t) < limit.window);
            
            if entry.requests.len() < limit.requests {
                // We can make a request
                entry.requests.push(now);
                break;
            }
            
            // We need to wait
            drop(state);
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Adds a rate limit for the specified key
    pub fn add_limit(&mut self, key: &str, requests: usize, window: Duration) {
        self.limits.insert(
            key.to_string(),
            RateLimit {
                requests,
                window,
            },
        );
    }
} 