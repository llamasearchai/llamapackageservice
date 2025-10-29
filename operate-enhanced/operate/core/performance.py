"""Performance optimization module with caching and parallel execution."""
import asyncio
import hashlib
import time
from collections import OrderedDict
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Callable, Dict, List, Optional, Set, Tuple
import logging

import numpy as np
from PIL import Image
import io

from ..interfaces import Action, ActionType, OperationResult


logger = logging.getLogger(__name__)


@dataclass
class CacheEntry:
    """Cache entry with metadata."""
    key: str
    value: Any
    timestamp: datetime
    hit_count: int = 0
    size_bytes: int = 0
    ttl: Optional[int] = None
    
    def is_expired(self) -> bool:
        """Check if cache entry is expired."""
        if not self.ttl:
            return False
        return datetime.utcnow() > self.timestamp + timedelta(seconds=self.ttl)


@dataclass
class PerformanceMetrics:
    """Performance metrics for operations."""
    operation_count: int = 0
    total_duration: float = 0.0
    cache_hits: int = 0
    cache_misses: int = 0
    parallel_executions: int = 0
    failed_operations: int = 0
    
    @property
    def average_duration(self) -> float:
        """Get average operation duration."""
        return self.total_duration / self.operation_count if self.operation_count > 0 else 0.0
    
    @property
    def cache_hit_rate(self) -> float:
        """Get cache hit rate."""
        total = self.cache_hits + self.cache_misses
        return self.cache_hits / total if total > 0 else 0.0


class LRUCache:
    """Least Recently Used cache implementation."""
    
    def __init__(self, max_size: int = 1000, max_memory_mb: int = 100):
        self.max_size = max_size
        self.max_memory_bytes = max_memory_mb * 1024 * 1024
        self._cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._total_size = 0
        self._lock = asyncio.Lock()
        
    async def get(self, key: str) -> Optional[Any]:
        """Get value from cache."""
        async with self._lock:
            if key not in self._cache:
                return None
                
            entry = self._cache[key]
            
            # Check expiration
            if entry.is_expired():
                del self._cache[key]
                self._total_size -= entry.size_bytes
                return None
                
            # Move to end (most recently used)
            self._cache.move_to_end(key)
            entry.hit_count += 1
            
            return entry.value
            
    async def set(self, key: str, value: Any, ttl: Optional[int] = None):
        """Set value in cache."""
        # Estimate size
        size_bytes = self._estimate_size(value)
        
        async with self._lock:
            # Remove old entry if exists
            if key in self._cache:
                self._total_size -= self._cache[key].size_bytes
                
            # Add new entry
            entry = CacheEntry(
                key=key,
                value=value,
                timestamp=datetime.utcnow(),
                size_bytes=size_bytes,
                ttl=ttl
            )
            
            self._cache[key] = entry
            self._cache.move_to_end(key)
            self._total_size += size_bytes
            
            # Evict if necessary
            await self._evict_if_needed()
            
    async def _evict_if_needed(self):
        """Evict entries if cache is too large."""
        while len(self._cache) > self.max_size or self._total_size > self.max_memory_bytes:
            if not self._cache:
                break
                
            # Remove least recently used
            key, entry = self._cache.popitem(last=False)
            self._total_size -= entry.size_bytes
            logger.debug(f"Evicted cache entry: {key}")
            
    def _estimate_size(self, value: Any) -> int:
        """Estimate memory size of value."""
        if isinstance(value, bytes):
            return len(value)
        elif isinstance(value, str):
            return len(value.encode())
        elif isinstance(value, (list, dict)):
            # Rough estimate
            return len(str(value))
        else:
            return 64  # Default size
            
    async def clear(self):
        """Clear all cache entries."""
        async with self._lock:
            self._cache.clear()
            self._total_size = 0
            
    async def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        async with self._lock:
            total_hits = sum(e.hit_count for e in self._cache.values())
            return {
                "entries": len(self._cache),
                "total_size_mb": self._total_size / (1024 * 1024),
                "total_hits": total_hits,
                "avg_hit_count": total_hits / len(self._cache) if self._cache else 0
            }


class ScreenshotOptimizer:
    """Optimize screenshot operations."""
    
    def __init__(self, cache: LRUCache):
        self.cache = cache
        self._last_screenshot: Optional[bytes] = None
        self._last_hash: Optional[str] = None
        
    async def capture_optimized(self, capture_func: Callable) -> bytes:
        """Capture screenshot with optimization."""
        # Check if screen might have changed
        cache_key = "screenshot_current"
        cached = await self.cache.get(cache_key)
        
        if cached:
            # Verify if still valid (quick check)
            return cached
            
        # Capture new screenshot
        screenshot = await capture_func()
        
        # Compress if needed
        compressed = await self._compress_screenshot(screenshot)
        
        # Cache for short duration
        await self.cache.set(cache_key, compressed, ttl=2)
        
        return compressed
        
    async def _compress_screenshot(self, screenshot: bytes) -> bytes:
        """Compress screenshot for efficiency."""
        try:
            # Open image
            img = Image.open(io.BytesIO(screenshot))
            
            # Reduce quality for non-critical operations
            output = io.BytesIO()
            img.save(output, format='JPEG', quality=85, optimize=True)
            
            compressed = output.getvalue()
            logger.debug(f"Compressed screenshot from {len(screenshot)} to {len(compressed)} bytes")
            
            return compressed
        except Exception as e:
            logger.warning(f"Failed to compress screenshot: {str(e)}")
            return screenshot
            
    async def detect_changes(self, screenshot: bytes) -> bool:
        """Detect if screen has changed significantly."""
        current_hash = hashlib.md5(screenshot).hexdigest()
        
        if self._last_hash and current_hash == self._last_hash:
            return False
            
        self._last_hash = current_hash
        return True


class ModelResponseCache:
    """Cache for AI model responses."""
    
    def __init__(self, cache: LRUCache):
        self.cache = cache
        
    async def get_cached_response(self, screenshot: bytes, objective: str) -> Optional[Action]:
        """Get cached model response."""
        # Create cache key from inputs
        screenshot_hash = hashlib.md5(screenshot).hexdigest()[:8]
        objective_hash = hashlib.md5(objective.encode()).hexdigest()[:8]
        cache_key = f"model_response_{screenshot_hash}_{objective_hash}"
        
        return await self.cache.get(cache_key)
        
    async def cache_response(self, screenshot: bytes, objective: str, response: Action):
        """Cache model response."""
        screenshot_hash = hashlib.md5(screenshot).hexdigest()[:8]
        objective_hash = hashlib.md5(objective.encode()).hexdigest()[:8]
        cache_key = f"model_response_{screenshot_hash}_{objective_hash}"
        
        # Cache for 5 minutes
        await self.cache.set(cache_key, response, ttl=300)


class ParallelExecutor:
    """Execute operations in parallel when possible."""
    
    def __init__(self, max_workers: int = 4):
        self.max_workers = max_workers
        self._semaphore = asyncio.Semaphore(max_workers)
        self._running_tasks: Set[asyncio.Task] = set()
        
    async def execute_parallel(
        self,
        operations: List[Tuple[Callable, tuple, dict]],
        timeout: Optional[float] = None
    ) -> List[Any]:
        """Execute operations in parallel."""
        tasks = []
        
        for func, args, kwargs in operations:
            task = asyncio.create_task(self._execute_with_semaphore(func, args, kwargs))
            tasks.append(task)
            self._running_tasks.add(task)
            
        try:
            if timeout:
                results = await asyncio.wait_for(
                    asyncio.gather(*tasks, return_exceptions=True),
                    timeout=timeout
                )
            else:
                results = await asyncio.gather(*tasks, return_exceptions=True)
                
            return results
        finally:
            # Clean up
            for task in tasks:
                self._running_tasks.discard(task)
                
    async def _execute_with_semaphore(self, func: Callable, args: tuple, kwargs: dict) -> Any:
        """Execute function with semaphore."""
        async with self._semaphore:
            return await func(*args, **kwargs)
            
    def get_active_count(self) -> int:
        """Get number of active parallel executions."""
        return len(self._running_tasks)


class PerformanceOptimizer:
    """Main performance optimization coordinator."""
    
    def __init__(self, cache_size: int = 1000, max_memory_mb: int = 100, max_workers: int = 4):
        self.cache = LRUCache(cache_size, max_memory_mb)
        self.screenshot_optimizer = ScreenshotOptimizer(self.cache)
        self.model_cache = ModelResponseCache(self.cache)
        self.parallel_executor = ParallelExecutor(max_workers)
        self.metrics = PerformanceMetrics()
        
    async def optimize_screenshot_capture(self, capture_func: Callable) -> bytes:
        """Capture screenshot with optimizations."""
        start_time = time.time()
        
        try:
            screenshot = await self.screenshot_optimizer.capture_optimized(capture_func)
            self.metrics.cache_hits += 1
            return screenshot
        except:
            self.metrics.cache_misses += 1
            screenshot = await capture_func()
            return screenshot
        finally:
            duration = time.time() - start_time
            self.metrics.operation_count += 1
            self.metrics.total_duration += duration
            
    async def get_model_response(
        self,
        screenshot: bytes,
        objective: str,
        model_func: Callable
    ) -> Action:
        """Get model response with caching."""
        # Check cache first
        cached = await self.model_cache.get_cached_response(screenshot, objective)
        if cached:
            self.metrics.cache_hits += 1
            logger.debug("Using cached model response")
            return cached
            
        self.metrics.cache_misses += 1
        
        # Get fresh response
        response = await model_func(screenshot, objective)
        
        # Cache for future use
        await self.model_cache.cache_response(screenshot, objective, response)
        
        return response
        
    async def execute_operations_optimized(
        self,
        operations: List[Tuple[Callable, tuple, dict]],
        allow_parallel: bool = True
    ) -> List[Any]:
        """Execute operations with optimization."""
        if not allow_parallel or len(operations) == 1:
            # Sequential execution
            results = []
            for func, args, kwargs in operations:
                result = await func(*args, **kwargs)
                results.append(result)
            return results
            
        # Parallel execution
        self.metrics.parallel_executions += 1
        return await self.parallel_executor.execute_parallel(operations)
        
    async def batch_similar_operations(
        self,
        operations: List[Action]
    ) -> List[List[Action]]:
        """Batch similar operations for efficiency."""
        batches: List[List[Action]] = []
        current_batch: List[Action] = []
        
        for op in operations:
            if not current_batch:
                current_batch.append(op)
            elif self._can_batch(current_batch[-1], op):
                current_batch.append(op)
            else:
                batches.append(current_batch)
                current_batch = [op]
                
        if current_batch:
            batches.append(current_batch)
            
        return batches
        
    def _can_batch(self, op1: Action, op2: Action) -> bool:
        """Check if two operations can be batched."""
        # Same type operations on different targets can be batched
        if op1.type == op2.type and op1.target != op2.target:
            return True
            
        # Screenshot operations can always be batched
        if op1.type == ActionType.SCREENSHOT and op2.type == ActionType.SCREENSHOT:
            return True
            
        return False
        
    async def get_metrics(self) -> Dict[str, Any]:
        """Get performance metrics."""
        cache_stats = await self.cache.get_stats()
        
        return {
            "operations": {
                "total": self.metrics.operation_count,
                "failed": self.metrics.failed_operations,
                "average_duration_ms": self.metrics.average_duration * 1000,
                "parallel_executions": self.metrics.parallel_executions
            },
            "cache": {
                **cache_stats,
                "hit_rate": self.metrics.cache_hit_rate,
                "total_hits": self.metrics.cache_hits,
                "total_misses": self.metrics.cache_misses
            },
            "parallel": {
                "active_workers": self.parallel_executor.get_active_count(),
                "max_workers": self.parallel_executor.max_workers
            }
        }
        
    async def optimize_operation_sequence(
        self,
        operations: List[Action]
    ) -> List[Action]:
        """Optimize a sequence of operations."""
        optimized = []
        
        # Remove redundant operations
        seen_targets = set()
        for op in operations:
            if op.type == ActionType.CLICK:
                # Skip duplicate clicks on same target
                if op.target in seen_targets:
                    continue
                seen_targets.add(op.target)
                
            optimized.append(op)
            
        # Reorder for efficiency (screenshots first, then actions)
        screenshots = [op for op in optimized if op.type == ActionType.SCREENSHOT]
        others = [op for op in optimized if op.type != ActionType.SCREENSHOT]
        
        return screenshots + others
        
    async def clear_cache(self):
        """Clear all caches."""
        await self.cache.clear()
        self.metrics = PerformanceMetrics()
        logger.info("Cleared all performance caches")