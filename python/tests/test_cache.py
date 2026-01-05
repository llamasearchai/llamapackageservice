"""
Tests for cache module.
"""

import pytest
from pathlib import Path
import tempfile
import time
import asyncio

from llamapackageservice.cache import Cache, FileCache, CacheEntry


class TestCacheEntry:
    """Tests for CacheEntry."""
    
    def test_cache_entry_creation(self):
        """Test creating a cache entry."""
        entry = CacheEntry(data="test data", ttl=60)
        
        assert entry.data == "test data"
        assert entry.ttl == 60
        assert entry.created_at is not None
    
    def test_cache_entry_is_valid(self):
        """Test checking if cache entry is valid."""
        entry = CacheEntry(data="test", ttl=60)
        assert entry.is_valid() is True
    
    def test_cache_entry_expired(self):
        """Test expired cache entry."""
        entry = CacheEntry(data="test", ttl=0)
        time.sleep(0.1)
        assert entry.is_valid() is False


class TestCache:
    """Tests for in-memory Cache."""
    
    def test_cache_get_set(self):
        """Test basic get/set operations."""
        cache: Cache[str] = Cache(default_ttl=60)
        
        cache.set("key1", "value1")
        assert cache.get("key1") == "value1"
    
    def test_cache_miss(self):
        """Test cache miss returns None."""
        cache: Cache[str] = Cache(default_ttl=60)
        
        assert cache.get("nonexistent") is None
    
    def test_cache_delete(self):
        """Test deleting from cache."""
        cache: Cache[str] = Cache(default_ttl=60)
        
        cache.set("key1", "value1")
        cache.delete("key1")
        assert cache.get("key1") is None
    
    def test_cache_clear(self):
        """Test clearing cache."""
        cache: Cache[str] = Cache(default_ttl=60)
        
        cache.set("key1", "value1")
        cache.set("key2", "value2")
        cache.clear()
        
        assert cache.get("key1") is None
        assert cache.get("key2") is None
    
    def test_cache_expiration(self):
        """Test cache entry expiration."""
        cache: Cache[str] = Cache(default_ttl=0)
        
        cache.set("key1", "value1")
        time.sleep(0.1)
        assert cache.get("key1") is None
    
    def test_cache_custom_ttl(self):
        """Test custom TTL for individual entries."""
        cache: Cache[str] = Cache(default_ttl=60)
        
        cache.set("key1", "value1", ttl=0)
        time.sleep(0.1)
        assert cache.get("key1") is None


class TestFileCache:
    """Tests for FileCache."""
    
    @pytest.fixture
    def temp_cache_dir(self):
        """Create a temporary cache directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)
    
    @pytest.mark.asyncio
    async def test_file_cache_get_set(self, temp_cache_dir):
        """Test basic file cache get/set."""
        cache = FileCache(temp_cache_dir, default_ttl=60)
        
        await cache.set("key1", "value1")
        result = await cache.get("key1")
        assert result == "value1"
    
    @pytest.mark.asyncio
    async def test_file_cache_miss(self, temp_cache_dir):
        """Test file cache miss."""
        cache = FileCache(temp_cache_dir, default_ttl=60)
        
        result = await cache.get("nonexistent")
        assert result is None
    
    @pytest.mark.asyncio
    async def test_file_cache_complex_data(self, temp_cache_dir):
        """Test caching complex data structures."""
        cache = FileCache(temp_cache_dir, default_ttl=60)
        
        data = {
            "name": "test",
            "items": [1, 2, 3],
            "nested": {"key": "value"},
        }
        
        await cache.set("complex", data)
        result = await cache.get("complex")
        assert result == data
    
    @pytest.mark.asyncio
    async def test_file_cache_delete(self, temp_cache_dir):
        """Test deleting from file cache."""
        cache = FileCache(temp_cache_dir, default_ttl=60)
        
        await cache.set("key1", "value1")
        await cache.delete("key1")
        result = await cache.get("key1")
        assert result is None
    
    @pytest.mark.asyncio
    async def test_file_cache_clear(self, temp_cache_dir):
        """Test clearing file cache."""
        cache = FileCache(temp_cache_dir, default_ttl=60)
        
        await cache.set("key1", "value1")
        await cache.set("key2", "value2")
        await cache.clear()
        
        assert await cache.get("key1") is None
        assert await cache.get("key2") is None
    
    @pytest.mark.asyncio
    async def test_file_cache_persistence(self, temp_cache_dir):
        """Test that file cache persists data."""
        cache1 = FileCache(temp_cache_dir, default_ttl=60)
        await cache1.set("key1", "value1")
        
        # Create new cache instance pointing to same directory
        cache2 = FileCache(temp_cache_dir, default_ttl=60)
        result = await cache2.get("key1")
        assert result == "value1"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
