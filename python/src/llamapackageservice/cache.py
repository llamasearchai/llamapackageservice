"""
Caching mechanisms to improve performance.

This module provides both file-based and in-memory caching for
downloaded content and processed results.
"""

from pathlib import Path
from datetime import datetime, timedelta
from typing import TypeVar, Generic, Optional, Dict, Any
import hashlib
import json
import asyncio
from dataclasses import dataclass, field

import aiofiles
import aiofiles.os

from .error import CacheError


@dataclass
class CacheEntry:
    """Represents a single entry in the file cache."""
    
    url: str
    path: Path
    timestamp: datetime
    ttl: timedelta
    
    def is_expired(self) -> bool:
        """Check if this cache entry has expired."""
        return datetime.now() - self.timestamp > self.ttl
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "url": self.url,
            "path": str(self.path),
            "timestamp": self.timestamp.isoformat(),
            "ttl_seconds": self.ttl.total_seconds(),
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CacheEntry":
        """Create from dictionary."""
        return cls(
            url=data["url"],
            path=Path(data["path"]),
            timestamp=datetime.fromisoformat(data["timestamp"]),
            ttl=timedelta(seconds=data["ttl_seconds"]),
        )


class FileCache:
    """A file-based cache for storing downloaded or generated content."""
    
    def __init__(self, cache_dir: Path, default_ttl: timedelta = timedelta(hours=24)):
        self.cache_dir = Path(cache_dir)
        self.default_ttl = default_ttl
    
    def _get_safe_key(self, key: str) -> str:
        """Create a safe filename from the key."""
        # Replace special characters with underscores
        safe_key = key.replace("/", "_").replace(":", "_").replace("?", "_").replace("&", "_").replace("=", "_")
        # If the key is too long, hash it
        if len(safe_key) > 200:
            safe_key = hashlib.sha256(key.encode()).hexdigest()
        return safe_key
    
    def _get_path(self, key: str) -> Path:
        """Get the cache file path for a key."""
        safe_key = self._get_safe_key(key)
        return self.cache_dir / safe_key
    
    def _get_meta_path(self, key: str) -> Path:
        """Get the metadata file path for a key."""
        return self._get_path(key).with_suffix(".meta")
    
    async def get(self, key: str) -> Optional[CacheEntry]:
        """
        Retrieve a cached entry by its key if it exists and is not expired.
        
        Args:
            key: The cache key to look up
            
        Returns:
            The cached entry if found and not expired, None otherwise
        """
        meta_path = self._get_meta_path(key)
        
        if not await aiofiles.os.path.exists(meta_path):
            return None
        
        try:
            async with aiofiles.open(meta_path, 'r') as f:
                content = await f.read()
            
            entry = CacheEntry.from_dict(json.loads(content))
            
            # Check if entry is expired
            if entry.is_expired():
                # Entry is expired, remove it
                await self.remove(key)
                return None
            
            return entry
        except Exception as e:
            raise CacheError(f"Failed to read cache entry: {e}", source=e)
    
    async def set(
        self,
        key: str,
        value: bytes,
        ttl: Optional[timedelta] = None
    ) -> Path:
        """
        Store a value in the cache with the specified key and TTL.
        
        Args:
            key: The cache key
            value: The content to cache
            ttl: Optional TTL, defaults to default_ttl
            
        Returns:
            Path to the cached file
        """
        path = self._get_path(key)
        meta_path = self._get_meta_path(key)
        
        # Ensure cache directory exists
        if path.parent:
            await aiofiles.os.makedirs(path.parent, exist_ok=True)
        
        try:
            # Write the value
            async with aiofiles.open(path, 'wb') as f:
                await f.write(value)
            
            # Create and save the entry metadata
            entry = CacheEntry(
                url=key,
                path=path,
                timestamp=datetime.now(),
                ttl=ttl or self.default_ttl,
            )
            
            async with aiofiles.open(meta_path, 'w') as f:
                await f.write(json.dumps(entry.to_dict()))
            
            return path
        except Exception as e:
            raise CacheError(f"Failed to write cache entry: {e}", source=e)
    
    async def remove(self, key: str) -> bool:
        """Remove an entry from the cache."""
        path = self._get_path(key)
        meta_path = self._get_meta_path(key)
        removed = False
        
        try:
            if await aiofiles.os.path.exists(path):
                await aiofiles.os.remove(path)
                removed = True
            if await aiofiles.os.path.exists(meta_path):
                await aiofiles.os.remove(meta_path)
                removed = True
        except Exception:
            pass
        
        return removed
    
    async def clear(self) -> int:
        """Clear all entries from the cache. Returns number of entries removed."""
        count = 0
        try:
            if await aiofiles.os.path.exists(self.cache_dir):
                for item in self.cache_dir.iterdir():
                    await aiofiles.os.remove(item)
                    count += 1
        except Exception:
            pass
        return count


T = TypeVar('T')


class Cache(Generic[T]):
    """A generic in-memory cache for any type."""
    
    def __init__(self, ttl: timedelta = timedelta(hours=1)):
        self._store: Dict[str, tuple[T, datetime]] = {}
        self._ttl = ttl
        self._lock = asyncio.Lock()
    
    async def get(self, key: str) -> Optional[T]:
        """
        Retrieve a value from the cache by its key.
        
        Args:
            key: The cache key
            
        Returns:
            The cached value if found and not expired, None otherwise
        """
        async with self._lock:
            if key not in self._store:
                return None
            
            value, timestamp = self._store[key]
            
            # Check if expired
            if datetime.now() - timestamp > self._ttl:
                del self._store[key]
                return None
            
            return value
    
    async def set(self, key: str, value: T) -> None:
        """
        Store a value in the cache with the specified key.
        
        Args:
            key: The cache key
            value: The value to store
        """
        async with self._lock:
            self._store[key] = (value, datetime.now())
    
    async def remove(self, key: str) -> bool:
        """
        Remove an entry from the cache by its key.
        
        Args:
            key: The cache key
            
        Returns:
            True if the entry was removed, False if it didn't exist
        """
        async with self._lock:
            if key in self._store:
                del self._store[key]
                return True
            return False
    
    async def clear(self) -> None:
        """Clear all entries from the cache."""
        async with self._lock:
            self._store.clear()
    
    async def cleanup_expired(self) -> int:
        """
        Remove all expired entries from the cache.
        
        Returns:
            Number of entries removed
        """
        async with self._lock:
            now = datetime.now()
            expired_keys = [
                key for key, (_, timestamp) in self._store.items()
                if now - timestamp > self._ttl
            ]
            for key in expired_keys:
                del self._store[key]
            return len(expired_keys)
    
    def __len__(self) -> int:
        """Return the number of entries in the cache."""
        return len(self._store)


class StringCache(Cache[str]):
    """Specialized cache for string values."""
    pass


class BytesCache(Cache[bytes]):
    """Specialized cache for bytes values."""
    pass
