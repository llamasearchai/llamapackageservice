use std::path::Path;
use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use crate::error::Result;

pub trait CacheTrait<K, V> {
    fn get(&mut self, key: &K) -> Option<&V>;
    fn insert(&mut self, key: K, value: V);
}

pub struct MemoryCache<K: Hash + Eq, V> {
    inner: LruCache<K, V>,
}

impl<K: Hash + Eq, V> MemoryCache<K, V> {
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

pub struct FileCache {
    dir: std::path::PathBuf,
}

impl FileCache {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        Ok(Self {
            dir: cache_dir.to_path_buf(),
        })
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let path = self.dir.join(key);
        std::fs::read_to_string(path).ok()
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let path = self.dir.join(key);
        std::fs::write(path, value)?;
        Ok(())
    }
}