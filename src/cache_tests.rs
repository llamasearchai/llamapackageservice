#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_generic_cache() {
        let cache: Cache<i32> = Cache::new(Duration::from_millis(100));
        
        // Test basic operations
        cache.set("key1", 42).await;
        assert_eq!(cache.get("key1").await, Some(42));
        assert_eq!(cache.len().await, 1);
        assert!(!cache.is_empty().await);
        assert!(cache.contains_key("key1").await);
        assert!(!cache.contains_key("key2").await);
        
        // Test TTL expiration
        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get("key1").await, None);
        
        // Cleanup should remove expired entries
        cache.set("key2", 100).await;
        sleep(Duration::from_millis(150)).await;
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert_eq!(cache.len().await, 0);
        
        // Test remove
        cache.set("key3", 200).await;
        assert!(cache.remove("key3").await);
        assert!(!cache.remove("key3").await);
        assert_eq!(cache.len().await, 0);
        
        // Test clear
        cache.set("key4", 300).await;
        cache.set("key5", 400).await;
        cache.clear().await;
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_string_cache() -> Result<(), ProcessorError> {
        let temp_dir = tempdir().unwrap();
        let mut cache = StringCache::new(temp_dir.path())?;
        
        // Test TTL configuration
        cache.set_ttl(Duration::from_millis(100));
        
        // Test basic operations
        cache.set("key1", "value1".to_string()).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        assert_eq!(cache.len().await, 1);
        assert!(!cache.is_empty().await);
        assert!(cache.contains_key("key1").await);
        assert!(!cache.contains_key("key2").await);
        
        // Test TTL expiration
        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get("key1").await, None);
        
        // Test save and load
        cache.set("key2", "value2".to_string()).await;
        cache.save().await?;
        
        // Create a new cache instance and load
        let cache2 = StringCache::new(temp_dir.path())?;
        cache2.load().await?;
        assert_eq!(cache2.get("key2").await, Some("value2".to_string()));
        
        // Test remove
        assert!(cache2.remove("key2").await);
        assert!(!cache2.contains_key("key2").await);
        
        // Test clear
        cache2.set("key3", "value3".to_string()).await;
        cache2.set("key4", "value4".to_string()).await;
        cache2.clear().await;
        assert_eq!(cache2.len().await, 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_string_cache_persistence_with_ttl() -> Result<(), ProcessorError> {
        let temp_dir = tempdir().unwrap();
        
        // Create cache with short TTL
        let cache = StringCache::with_ttl(temp_dir.path(), Duration::from_millis(200))?;
        
        // Add some entries
        cache.set("expires_soon", "value1".to_string()).await;
        sleep(Duration::from_millis(100)).await;
        cache.set("expires_later", "value2".to_string()).await;
        
        // Save to disk
        cache.save().await?;
        
        // Wait for first entry to expire
        sleep(Duration::from_millis(150)).await;
        
        // Load in a new cache
        let cache2 = StringCache::with_ttl(temp_dir.path(), Duration::from_millis(200))?;
        cache2.load().await?;
        
        // First entry should have expired, second should still be valid
        assert_eq!(cache2.get("expires_soon").await, None);
        assert_eq!(cache2.get("expires_later").await, Some("value2".to_string()));
        
        Ok(())
    }
}
