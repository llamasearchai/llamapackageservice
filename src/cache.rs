use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, Instant};
use serde::{Serialize, Deserialize};
use crate::error::{ProcessorError, Result};
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs as tokio_fs;
use chrono;
use std::io::Read;

/// Represents a single entry in the file cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The URL or identifier for this cache entry
    pub url: String,
    /// The file path where the cached content is stored
    pub path: PathBuf,
    /// When this entry was created or last updated
    pub timestamp: SystemTime,
    /// How long this entry should remain valid
    pub ttl: Duration,
}

/// A file-based cache for storing downloaded or generated content
#[derive(Debug)]
pub struct FileCache {
    cache_dir: PathBuf,
    default_ttl: Duration,
}

impl FileCache {
    /// Creates a new FileCache with the specified cache directory and default TTL
    pub fn new(cache_dir: PathBuf, default_ttl: Duration) -> Self {
        Self {
            cache_dir,
            default_ttl,
        }
    }
    
    /// Retrieves a cached entry by its key if it exists and is not expired
    pub async fn get(&self, key: &str) -> Result<Option<CacheEntry>> {
        let path = self.get_path(key);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        let entry: CacheEntry = serde_json::from_str(&content)?;
        
        // Check if entry is expired
        let now = SystemTime::now();
        if now.duration_since(entry.timestamp).unwrap_or_default() > entry.ttl {
            // Entry is expired, remove it
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| ProcessorError::IO(e))?;
            return Ok(None);
        }
        
        Ok(Some(entry))
    }
    
    /// Stores a value in the cache with the specified key and TTL
    pub async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<PathBuf> {
        let path = self.get_path(key);
        
        // Ensure cache directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ProcessorError::IO(e))?;
        }
        
        // Write the value
        tokio::fs::write(&path, value)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        // Create and save the entry metadata
        let entry = CacheEntry {
            url: key.to_string(),
            path: path.clone(),
            timestamp: SystemTime::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        };
        
        let meta_path = self.get_meta_path(key);
        tokio::fs::write(&meta_path, serde_json::to_string(&entry)?)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(path)
    }
    
    fn get_path(&self, key: &str) -> PathBuf {
        // Create a safe filename from the key
        let safe_key = key.replace(['/', ':', '?', '&', '='], "_");
        self.cache_dir.join(safe_key)
    }
    
    fn get_meta_path(&self, key: &str) -> PathBuf {
        let mut path = self.get_path(key);
        path.set_extension("meta");
        path
    }
}

/// A generic in-memory cache for any serializable type
#[derive(Debug)]
pub struct Cache<T> {
    store: Arc<RwLock<HashMap<String, (T, Instant)>>>,
    ttl: Duration,
}

impl<T: Clone + Send + Sync + 'static> Cache<T> {
    /// Creates a new in-memory cache with the specified TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Retrieves a value from the cache by its key
    pub async fn get(&self, key: &str) -> Option<T> {
        let store = self.store.read().await;
        store.get(key)
            .filter(|(_, time)| time.elapsed() < self.ttl)
            .map(|(value, _)| value.clone())
    }

    /// Stores a value in the cache with the specified key
    pub async fn set(&self, key: &str, value: T) {
        let mut store = self.store.write().await;
        store.insert(key.to_string(), (value, Instant::now()));
    }

    /// Removes an entry from the cache by its key
    pub async fn remove(&self, key: &str) -> bool {
        let mut store = self.store.write().await;
        store.remove(key).is_some()
    }

    /// Clears all entries from the cache
    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        store.clear();
    }

    /// Checks if the cache contains the specified key
    pub async fn contains_key(&self, key: &str) -> bool {
        let store = self.store.read().await;
        store.contains_key(key)
    }

    /// Returns the number of entries in the cache
    pub async fn len(&self) -> usize {
        let store = self.store.read().await;
        store.len()
    }

    /// Checks if the cache is empty
    pub async fn is_empty(&self) -> bool {
        let store = self.store.read().await;
        store.is_empty()
    }
    
    /// Removes expired entries from the cache and returns the count of removed entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut store = self.store.write().await;
        let before_len = store.len();
        store.retain(|_, (_, time)| time.elapsed() < self.ttl);
        before_len - store.len()
    }
    
    /// Sets a new TTL for the cache
    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCacheEntry {
    value: String,
    created_at: u64,
}

/// A specialized cache for storing string values with persistent storage
#[derive(Debug)]
pub struct StringCache {
    store: Arc<RwLock<HashMap<String, (String, Instant)>>>,
    ttl: Duration,
    cache_dir: PathBuf,
}

impl StringCache {
    /// Creates a new StringCache with the specified cache directory
    pub async fn new(cache_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(3600), // Default 1 hour TTL
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Creates a new StringCache with the specified cache directory and TTL
    pub async fn with_ttl(cache_dir: &Path, ttl: Duration) -> Result<Self> {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Sets a new TTL for the cache
    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }

    /// Retrieves a value from the cache by its key
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let path = self.get_path(key);
        
        if path.exists() {
            // Use tokio::fs instead of std::fs
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| ProcessorError::IO(e))?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    /// Stores a value in the cache with the specified key
    pub async fn set_value(&self, key: &str, value: String) {
        let mut store = self.store.write().await;
        store.insert(key.to_string(), (value, Instant::now()));
    }
    
    /// Removes an entry from the cache by its key
    pub async fn remove(&self, key: &str) -> bool {
        let mut store = self.store.write().await;
        store.remove(key).is_some()
    }

    /// Clears all entries from the cache
    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        store.clear();
    }

    /// Checks if the cache contains the specified key
    pub async fn contains_key(&self, key: &str) -> bool {
        let store = self.store.read().await;
        store.contains_key(key)
    }

    /// Returns the number of entries in the cache
    pub async fn len(&self) -> usize {
        let store = self.store.read().await;
        store.len()
    }

    /// Checks if the cache is empty
    pub async fn is_empty(&self) -> bool {
        let store = self.store.read().await;
        store.is_empty()
    }
    
    /// Removes expired entries from the cache and returns the count of removed entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut store = self.store.write().await;
        let before_len = store.len();
        store.retain(|_, (_, time)| time.elapsed() < self.ttl);
        before_len - store.len()
    }

    /// Saves the cache to disk
    pub async fn save(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("cache.json");
        
        let store = self.store.read().await;
        let serializable: HashMap<String, StoredCacheEntry> = store
            .iter()
            .map(|(k, (v, time))| {
                let created_at = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64 - time.elapsed().as_millis() as u64;
                
                (k.clone(), StoredCacheEntry {
                    value: v.clone(),
                    created_at,
                })
            })
            .collect();
            
        let content = serde_json::to_string(&serializable)
            .map_err(|e| ProcessorError::Config(e.to_string()))?;
        
        tokio::fs::write(cache_file, content)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(())
    }
    
    /// Loads the cache from disk
    pub async fn load(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("cache.json");
        
        if cache_file.exists() {
            let content = tokio::fs::read_to_string(&cache_file)
                .await
                .map_err(|e| ProcessorError::IO(e))?;
            
            let entries: HashMap<String, StoredCacheEntry> = serde_json::from_str(&content)
                .map_err(|e| ProcessorError::Config(e.to_string()))?;
            
            let now = Instant::now();
            let current_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
                
            let mut store = self.store.write().await;
            for (key, entry) in entries {
                // Calculate how old this entry is
                let age = current_time - entry.created_at;
                let age_duration = Duration::from_millis(age);
                
                // Create an Instant that represents when this entry was created
                let creation_instant = if age < now.elapsed().as_millis() as u64 {
                    // Entry is older than our program has been running
                    now - age_duration
                } else {
                    // Handle edge case - use a very old timestamp
                    now - now.elapsed()
                };
                
                store.insert(key, (entry.value, creation_instant));
            }
        }
        
        Ok(())
    }

    /// Invalidates a cache entry by its key
    pub async fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.get_path(key);
        
        if path.exists() {
            // Use tokio::fs instead of std::fs
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| ProcessorError::IO(e))?;
        }
        
        Ok(())
    }

    /// Stores a value in the cache with the specified key
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let path = self.get_path(key);
        
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                // Use tokio::fs instead of std::fs
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| ProcessorError::IO(e))?;
            }
        }
        
        // Use tokio::fs instead of std::fs
        tokio::fs::write(&path, value)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        Ok(())
    }

    /// Stores a value in the cache with the specified key and metadata
    pub async fn set_with_metadata(
        &self, 
        key: &str, 
        value: &str, 
        metadata: &CacheMetadata
    ) -> Result<()> {
        let path = self.get_path(key);
        let meta_path = self.get_metadata_path(key);
        
        let entry = CacheEntry {
            url: key.to_string(),
            path: path.clone(),
            timestamp: std::time::SystemTime::now(),
            ttl: metadata.ttl,
        };
        
        // Use tokio::fs instead of std::fs
        tokio::fs::write(&meta_path, serde_json::to_string(&entry)?)
            .await
            .map_err(|e| ProcessorError::IO(e))?;
        
        self.set(key, value).await
    }

    fn get_metadata_path(&self, key: &str) -> PathBuf {
        let mut path = self.get_path(key);
        path.set_extension("meta");
        path
    }

    fn get_path(&self, key: &str) -> PathBuf {
        // Create a safe filename from the key
        let safe_key = key.replace(['/', ':', '?', '&', '='], "_");
        self.cache_dir.join(safe_key)
    }
}

/// Metadata for cache entries with additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// How long this entry should remain valid
    pub ttl: Duration,
    /// The source of this cache entry
    pub source: String,
    /// When this entry was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CacheMetadata {
    /// Creates a new CacheMetadata with the specified TTL and source
    pub fn new(ttl: Duration, source: &str) -> Self {
        Self {
            ttl,
            source: source.to_string(),
            created_at: chrono::Utc::now(),
        }
    }
}