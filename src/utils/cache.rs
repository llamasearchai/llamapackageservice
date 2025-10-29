use std::path::Path;
use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use crate::error::Result;

/// Trait for cache implementations
pub trait CacheTrait<K, V> {
    /// Get a value from the cache
    fn get(&mut self, key: &K) -> Option<&V>;
    /// Insert a value into the cache
    fn insert(&mut self, key: K, value: V);
}

/// In-memory LRU cache with fixed capacity
pub struct MemoryCache<K: Hash + Eq, V> {
    inner: LruCache<K, V>,
}

impl<K: Hash + Eq, V> MemoryCache<K, V> {
    /// Create a new memory cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: LruCache::new(NonZeroUsize::new(capacity.max(1)).unwrap()),
        }
    }
}

impl<K: Hash + Eq, V> CacheTrait<K, V> for MemoryCache<K, V> {
    fn get(&mut self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    fn insert(&mut self, key: K, value: V) {
        self.inner.put(key, value);
    }
}

/// File-based cache that stores values on disk
pub struct FileCache {
    dir: std::path::PathBuf,
}

impl FileCache {
    /// Create a new file cache in the given directory
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        Ok(Self {
            dir: cache_dir.to_path_buf(),
        })
    }

    /// Get a value from the file cache
    pub fn get(&self, key: &str) -> Option<String> {
        let path = self.dir.join(key);
        std::fs::read_to_string(path).ok()
    }

    /// Set a value in the file cache
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let path = self.dir.join(key);
        std::fs::write(path, value)?;
        Ok(())
    }
}