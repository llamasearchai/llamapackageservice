"""
Performance Optimization Module for GitHub Repository Manager
"""
import asyncio
import aiofiles
import aiohttp
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor
from functools import lru_cache, wraps
from typing import Dict, List, Any, Optional, Callable, AsyncIterator
from pathlib import Path
import hashlib
import pickle
import time
import psutil
import resource
from dataclasses import dataclass
import numpy as np
from collections import defaultdict, OrderedDict
import heapq
import logging
import msgpack
import uvloop
from asyncio import Queue, Semaphore

logger = logging.getLogger(__name__)


@dataclass
class PerformanceMetrics:
    """Performance tracking metrics"""
    operation: str
    start_time: float
    end_time: float
    memory_before: int
    memory_after: int
    cpu_percent: float
    io_operations: int
    cache_hits: int
    cache_misses: int
    
    @property
    def duration(self) -> float:
        return self.end_time - self.start_time
    
    @property
    def memory_delta(self) -> int:
        return self.memory_after - self.memory_before


class AdaptiveCache:
    """Intelligent caching system with LRU + LFU hybrid approach"""
    
    def __init__(self, max_size: int = 1000, ttl: int = 3600):
        self.max_size = max_size
        self.ttl = ttl
        self.cache = OrderedDict()
        self.frequency = defaultdict(int)
        self.timestamps = {}
        self.stats = {'hits': 0, 'misses': 0}
        
    def get(self, key: str) -> Optional[Any]:
        """Get value from cache"""
        if key in self.cache:
            # Check TTL
            if time.time() - self.timestamps[key] > self.ttl:
                del self.cache[key]
                del self.timestamps[key]
                self.stats['misses'] += 1
                return None
            
            # Update access pattern
            self.frequency[key] += 1
            self.cache.move_to_end(key)
            self.stats['hits'] += 1
            return self.cache[key]
        
        self.stats['misses'] += 1
        return None
    
    def set(self, key: str, value: Any):
        """Set value in cache"""
        if key in self.cache:
            del self.cache[key]
        elif len(self.cache) >= self.max_size:
            # Evict based on combined LRU/LFU score
            self._evict()
        
        self.cache[key] = value
        self.timestamps[key] = time.time()
        self.frequency[key] = 1
    
    def _evict(self):
        """Evict items based on adaptive algorithm"""
        # Calculate scores (lower is better)
        scores = []
        current_time = time.time()
        
        for key in self.cache:
            age = current_time - self.timestamps[key]
            freq = self.frequency[key]
            # Combined score: age/frequency
            score = age / (freq + 1)
            heapq.heappush(scores, (-score, key))
        
        # Evict bottom 10%
        evict_count = max(1, int(self.max_size * 0.1))
        for _ in range(evict_count):
            if scores:
                _, key = heapq.heappop(scores)
                del self.cache[key]
                del self.timestamps[key]
                del self.frequency[key]
    
    @property
    def hit_rate(self) -> float:
        """Calculate cache hit rate"""
        total = self.stats['hits'] + self.stats['misses']
        return self.stats['hits'] / total if total > 0 else 0


class StreamingProcessor:
    """Process large files in streaming chunks"""
    
    def __init__(self, chunk_size: int = 1024 * 1024):  # 1MB chunks
        self.chunk_size = chunk_size
        self.buffer_pool = []
        self._initialize_buffers()
    
    def _initialize_buffers(self):
        """Pre-allocate buffers for better performance"""
        for _ in range(10):
            self.buffer_pool.append(bytearray(self.chunk_size))
    
    async def process_file_stream(self, file_path: Path, 
                                 processor: Callable[[bytes], Any]) -> AsyncIterator[Any]:
        """Stream process a file"""
        async with aiofiles.open(file_path, 'rb') as f:
            while True:
                # Get buffer from pool
                if self.buffer_pool:
                    buffer = self.buffer_pool.pop()
                else:
                    buffer = bytearray(self.chunk_size)
                
                # Read chunk
                chunk = await f.read(self.chunk_size)
                if not chunk:
                    # Return buffer to pool
                    self.buffer_pool.append(buffer)
                    break
                
                # Process chunk
                result = await processor(chunk)
                yield result
                
                # Return buffer to pool
                self.buffer_pool.append(buffer)


class ParallelExecutor:
    """Execute tasks in parallel with resource management"""
    
    def __init__(self, max_workers: int = None, max_memory_percent: float = 80):
        self.max_workers = max_workers or psutil.cpu_count()
        self.max_memory_percent = max_memory_percent
        self.thread_pool = ThreadPoolExecutor(max_workers=self.max_workers)
        self.process_pool = ProcessPoolExecutor(max_workers=self.max_workers)
        self.semaphore = Semaphore(self.max_workers)
        
    async def map_async(self, func: Callable, items: List[Any], 
                       use_processes: bool = False) -> List[Any]:
        """Map function over items in parallel"""
        # Check memory before starting
        if not self._check_memory():
            # Fall back to sequential processing
            return [await func(item) for item in items]
        
        executor = self.process_pool if use_processes else self.thread_pool
        loop = asyncio.get_event_loop()
        
        # Create tasks with semaphore
        tasks = []
        for item in items:
            task = self._execute_with_limit(loop, executor, func, item)
            tasks.append(task)
        
        return await asyncio.gather(*tasks)
    
    async def _execute_with_limit(self, loop, executor, func, item):
        """Execute with concurrency limit"""
        async with self.semaphore:
            if asyncio.iscoroutinefunction(func):
                return await func(item)
            else:
                return await loop.run_in_executor(executor, func, item)
    
    def _check_memory(self) -> bool:
        """Check if we have enough memory"""
        memory = psutil.virtual_memory()
        return memory.percent < self.max_memory_percent
    
    def shutdown(self):
        """Shutdown executors"""
        self.thread_pool.shutdown(wait=True)
        self.process_pool.shutdown(wait=True)


class ConnectionPool:
    """HTTP connection pooling for API requests"""
    
    def __init__(self, pool_size: int = 100, timeout: int = 30):
        self.pool_size = pool_size
        self.timeout = aiohttp.ClientTimeout(total=timeout)
        self.connector = None
        self.session = None
        
    async def initialize(self):
        """Initialize connection pool"""
        self.connector = aiohttp.TCPConnector(
            limit=self.pool_size,
            ttl_dns_cache=300,
            enable_cleanup_closed=True
        )
        self.session = aiohttp.ClientSession(
            connector=self.connector,
            timeout=self.timeout
        )
    
    async def request(self, method: str, url: str, **kwargs) -> aiohttp.ClientResponse:
        """Make HTTP request using pool"""
        if not self.session:
            await self.initialize()
        
        return await self.session.request(method, url, **kwargs)
    
    async def close(self):
        """Close connection pool"""
        if self.session:
            await self.session.close()


class QueryOptimizer:
    """Optimize database and API queries"""
    
    def __init__(self):
        self.query_cache = AdaptiveCache(max_size=5000)
        self.batch_queue = defaultdict(list)
        self.batch_size = 100
        self.batch_timeout = 0.1  # 100ms
        
    async def execute_query(self, query: str, params: Dict = None) -> Any:
        """Execute query with caching"""
        # Generate cache key
        cache_key = self._generate_cache_key(query, params)
        
        # Check cache
        cached = self.query_cache.get(cache_key)
        if cached is not None:
            return cached
        
        # Execute query
        result = await self._execute_raw_query(query, params)
        
        # Cache result
        self.query_cache.set(cache_key, result)
        
        return result
    
    async def batch_query(self, query_type: str, params: Dict) -> Any:
        """Batch similar queries together"""
        # Add to batch queue
        future = asyncio.Future()
        self.batch_queue[query_type].append((params, future))
        
        # Check if we should execute batch
        if len(self.batch_queue[query_type]) >= self.batch_size:
            await self._execute_batch(query_type)
        else:
            # Schedule batch execution
            asyncio.create_task(self._delayed_batch_execute(query_type))
        
        return await future
    
    async def _delayed_batch_execute(self, query_type: str):
        """Execute batch after timeout"""
        await asyncio.sleep(self.batch_timeout)
        if self.batch_queue[query_type]:
            await self._execute_batch(query_type)
    
    async def _execute_batch(self, query_type: str):
        """Execute batched queries"""
        batch = self.batch_queue[query_type]
        self.batch_queue[query_type] = []
        
        if not batch:
            return
        
        # Combine queries
        all_params = [params for params, _ in batch]
        
        # Execute batch query
        results = await self._execute_raw_batch_query(query_type, all_params)
        
        # Resolve futures
        for i, (_, future) in enumerate(batch):
            future.set_result(results[i])
    
    def _generate_cache_key(self, query: str, params: Dict) -> str:
        """Generate cache key for query"""
        key_data = f"{query}:{sorted(params.items()) if params else ''}"
        return hashlib.md5(key_data.encode()).hexdigest()
    
    async def _execute_raw_query(self, query: str, params: Dict) -> Any:
        """Execute actual query (to be implemented)"""
        # Placeholder for actual query execution
        await asyncio.sleep(0.01)  # Simulate query
        return {"query": query, "params": params, "result": "sample"}
    
    async def _execute_raw_batch_query(self, query_type: str, all_params: List[Dict]) -> List[Any]:
        """Execute batch query (to be implemented)"""
        # Placeholder for actual batch query execution
        await asyncio.sleep(0.02)  # Simulate batch query
        return [{"type": query_type, "params": params} for params in all_params]


class MemoryManager:
    """Manage memory usage and prevent OOM"""
    
    def __init__(self, max_memory_mb: int = 4096):
        self.max_memory_mb = max_memory_mb
        self.monitors = []
        
    def check_memory(self) -> Dict[str, Any]:
        """Check current memory usage"""
        process = psutil.Process()
        memory_info = process.memory_info()
        
        return {
            'rss_mb': memory_info.rss / 1024 / 1024,
            'vms_mb': memory_info.vms / 1024 / 1024,
            'percent': process.memory_percent(),
            'available_mb': psutil.virtual_memory().available / 1024 / 1024
        }
    
    def set_memory_limit(self):
        """Set process memory limit"""
        try:
            # Set soft and hard limits
            resource.setrlimit(
                resource.RLIMIT_AS,
                (self.max_memory_mb * 1024 * 1024, self.max_memory_mb * 1024 * 1024)
            )
        except Exception as e:
            logger.warning(f"Could not set memory limit: {e}")
    
    async def monitor_memory(self, callback: Callable = None):
        """Monitor memory usage continuously"""
        while True:
            memory = self.check_memory()
            
            # Check if approaching limit
            if memory['rss_mb'] > self.max_memory_mb * 0.8:
                logger.warning(f"Memory usage high: {memory['rss_mb']:.1f}MB")
                
                if callback:
                    await callback(memory)
                
                # Trigger garbage collection
                import gc
                gc.collect()
            
            await asyncio.sleep(10)  # Check every 10 seconds


class LazyLoader:
    """Lazy loading for large data structures"""
    
    def __init__(self, loader_func: Callable, cache_size: int = 100):
        self.loader_func = loader_func
        self.cache = OrderedDict()
        self.cache_size = cache_size
        self.loading = {}
        
    async def get(self, key: str) -> Any:
        """Get item with lazy loading"""
        # Check cache
        if key in self.cache:
            self.cache.move_to_end(key)
            return self.cache[key]
        
        # Check if already loading
        if key in self.loading:
            return await self.loading[key]
        
        # Load item
        future = asyncio.Future()
        self.loading[key] = future
        
        try:
            value = await self.loader_func(key)
            
            # Add to cache
            if len(self.cache) >= self.cache_size:
                self.cache.popitem(last=False)
            self.cache[key] = value
            
            future.set_result(value)
            return value
            
        finally:
            del self.loading[key]


def performance_monitor(func):
    """Decorator to monitor function performance"""
    @wraps(func)
    async def wrapper(*args, **kwargs):
        # Start metrics
        start_time = time.time()
        process = psutil.Process()
        memory_before = process.memory_info().rss
        cpu_before = process.cpu_percent()
        
        # Execute function
        result = await func(*args, **kwargs)
        
        # End metrics
        end_time = time.time()
        memory_after = process.memory_info().rss
        cpu_after = process.cpu_percent()
        
        # Log metrics
        metrics = PerformanceMetrics(
            operation=func.__name__,
            start_time=start_time,
            end_time=end_time,
            memory_before=memory_before,
            memory_after=memory_after,
            cpu_percent=(cpu_after - cpu_before),
            io_operations=0,  # Would need to track separately
            cache_hits=0,
            cache_misses=0
        )
        
        logger.debug(f"Performance: {func.__name__} took {metrics.duration:.3f}s, "
                    f"memory delta: {metrics.memory_delta/1024/1024:.1f}MB")
        
        return result
    
    return wrapper


class PerformanceOptimizer:
    """Central performance optimization manager"""
    
    def __init__(self):
        self.cache = AdaptiveCache(max_size=10000)
        self.streaming_processor = StreamingProcessor()
        self.parallel_executor = ParallelExecutor()
        self.connection_pool = ConnectionPool()
        self.query_optimizer = QueryOptimizer()
        self.memory_manager = MemoryManager()
        self.lazy_loader = LazyLoader(self._default_loader)
        self.metrics = []
        
    async def initialize(self):
        """Initialize all components"""
        # Use uvloop for better async performance
        asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
        
        await self.connection_pool.initialize()
        self.memory_manager.set_memory_limit()
        
        # Start memory monitor
        asyncio.create_task(self.memory_manager.monitor_memory(self._handle_memory_warning))
    
    async def optimize_file_processing(self, file_paths: List[Path], 
                                     processor: Callable) -> List[Any]:
        """Optimize processing of multiple files"""
        # Check cache first
        results = []
        uncached_paths = []
        
        for path in file_paths:
            cache_key = f"file:{path}:{path.stat().st_mtime}"
            cached = self.cache.get(cache_key)
            
            if cached:
                results.append(cached)
            else:
                uncached_paths.append(path)
        
        # Process uncached files in parallel
        if uncached_paths:
            # Use streaming for large files
            large_files = [p for p in uncached_paths if p.stat().st_size > 10 * 1024 * 1024]
            small_files = [p for p in uncached_paths if p not in large_files]
            
            # Process small files in parallel
            if small_files:
                small_results = await self.parallel_executor.map_async(
                    processor, small_files
                )
                results.extend(small_results)
                
                # Cache results
                for path, result in zip(small_files, small_results):
                    cache_key = f"file:{path}:{path.stat().st_mtime}"
                    self.cache.set(cache_key, result)
            
            # Stream process large files
            for path in large_files:
                result = await self._process_large_file(path, processor)
                results.append(result)
                
                # Cache result
                cache_key = f"file:{path}:{path.stat().st_mtime}"
                self.cache.set(cache_key, result)
        
        return results
    
    async def _process_large_file(self, path: Path, processor: Callable) -> Any:
        """Process large file with streaming"""
        results = []
        
        async for chunk_result in self.streaming_processor.process_file_stream(path, processor):
            results.append(chunk_result)
        
        # Combine results
        return self._combine_chunk_results(results)
    
    def _combine_chunk_results(self, results: List[Any]) -> Any:
        """Combine results from chunks"""
        # Implementation depends on the type of processing
        # This is a placeholder
        return results
    
    async def _default_loader(self, key: str) -> Any:
        """Default loader for lazy loading"""
        # Placeholder implementation
        await asyncio.sleep(0.1)
        return f"Loaded: {key}"
    
    async def _handle_memory_warning(self, memory_info: Dict):
        """Handle high memory usage"""
        logger.warning(f"High memory usage detected: {memory_info}")
        
        # Clear caches
        self.cache.cache.clear()
        
        # Trigger garbage collection
        import gc
        gc.collect()
        
        # Reduce parallel workers
        self.parallel_executor.max_workers = max(1, self.parallel_executor.max_workers // 2)
    
    def get_performance_report(self) -> Dict[str, Any]:
        """Get performance statistics"""
        return {
            'cache_hit_rate': self.cache.hit_rate,
            'cache_size': len(self.cache.cache),
            'memory_usage': self.memory_manager.check_memory(),
            'parallel_workers': self.parallel_executor.max_workers,
            'metrics': self.metrics[-100:]  # Last 100 metrics
        }
    
    async def cleanup(self):
        """Cleanup resources"""
        await self.connection_pool.close()
        self.parallel_executor.shutdown()