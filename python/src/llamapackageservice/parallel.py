"""
Parallel processing utilities.

Provides semaphore-based concurrency control for parallel task execution.
"""

import asyncio
from typing import List, TypeVar, Callable, Awaitable, Any
import logging

from .error import ProcessorError

logger = logging.getLogger(__name__)

T = TypeVar('T')


class ParallelProcessor:
    """
    Executes tasks in parallel with a specified concurrency limit.
    
    Uses asyncio semaphores to control the number of concurrent operations.
    """
    
    def __init__(self, max_concurrent: int = 5):
        """
        Create a new parallel processor with the specified concurrency limit.
        
        Args:
            max_concurrent: Maximum number of concurrent tasks
        """
        self.max_concurrent = max_concurrent
        self._semaphore = asyncio.Semaphore(max_concurrent)
    
    async def process(
        self,
        tasks: List[Callable[[], Awaitable[T]]],
    ) -> List[T | ProcessorError]:
        """
        Process a collection of async tasks concurrently.
        
        Args:
            tasks: List of async callables to execute
            
        Returns:
            List of results in the same order as tasks
        """
        async def execute_with_semaphore(
            task: Callable[[], Awaitable[T]],
            index: int,
        ) -> tuple[int, T | ProcessorError]:
            async with self._semaphore:
                try:
                    result = await task()
                    return (index, result)
                except ProcessorError as e:
                    return (index, e)
                except Exception as e:
                    logger.error(f"Task {index} failed: {e}")
                    return (index, ProcessorError(str(e)))
        
        # Create tasks with their indices
        indexed_tasks = [
            execute_with_semaphore(task, i)
            for i, task in enumerate(tasks)
        ]
        
        # Execute all tasks concurrently
        completed = await asyncio.gather(*indexed_tasks)
        
        # Sort results by original index
        sorted_results = sorted(completed, key=lambda x: x[0])
        
        return [result for _, result in sorted_results]
    
    async def map(
        self,
        func: Callable[[Any], Awaitable[T]],
        items: List[Any],
    ) -> List[T | ProcessorError]:
        """
        Apply an async function to each item in parallel.
        
        Args:
            func: Async function to apply
            items: Items to process
            
        Returns:
            List of results in the same order as items
        """
        tasks = [lambda item=item: func(item) for item in items]
        return await self.process(tasks)


class RateLimiter:
    """
    Rate limiter for controlling API request frequency.
    
    Uses a token bucket algorithm to limit requests per time window.
    """
    
    def __init__(
        self,
        requests_per_second: float = 10.0,
        burst_size: int = 20,
    ):
        """
        Create a new rate limiter.
        
        Args:
            requests_per_second: Maximum sustained request rate
            burst_size: Maximum burst size for temporary spikes
        """
        self.requests_per_second = requests_per_second
        self.burst_size = burst_size
        self._tokens = float(burst_size)
        self._last_update = asyncio.get_event_loop().time()
        self._lock = asyncio.Lock()
    
    async def acquire(self) -> None:
        """
        Acquire permission to make a request.
        
        Blocks until a token is available.
        """
        async with self._lock:
            now = asyncio.get_event_loop().time()
            time_passed = now - self._last_update
            self._tokens = min(
                self.burst_size,
                self._tokens + time_passed * self.requests_per_second
            )
            self._last_update = now
            
            if self._tokens < 1:
                wait_time = (1 - self._tokens) / self.requests_per_second
                await asyncio.sleep(wait_time)
                self._tokens = 0
            else:
                self._tokens -= 1


class TaskQueue:
    """
    Async task queue for background processing.
    """
    
    def __init__(self, max_workers: int = 5):
        """
        Create a new task queue.
        
        Args:
            max_workers: Maximum number of concurrent workers
        """
        self.max_workers = max_workers
        self._queue: asyncio.Queue = asyncio.Queue()
        self._workers: List[asyncio.Task] = []
        self._running = False
    
    async def start(self) -> None:
        """Start the worker tasks."""
        if self._running:
            return
        
        self._running = True
        self._workers = [
            asyncio.create_task(self._worker(i))
            for i in range(self.max_workers)
        ]
    
    async def stop(self) -> None:
        """Stop the worker tasks."""
        self._running = False
        
        for worker in self._workers:
            worker.cancel()
        
        await asyncio.gather(*self._workers, return_exceptions=True)
        self._workers = []
    
    async def submit(self, task: Callable[[], Awaitable[T]]) -> None:
        """
        Submit a task to the queue.
        
        Args:
            task: Async callable to execute
        """
        await self._queue.put(task)
    
    async def _worker(self, worker_id: int) -> None:
        """Worker coroutine that processes tasks from the queue."""
        while self._running:
            try:
                task = await asyncio.wait_for(self._queue.get(), timeout=1.0)
                try:
                    await task()
                except Exception as e:
                    logger.error(f"Worker {worker_id} task failed: {e}")
                finally:
                    self._queue.task_done()
            except asyncio.TimeoutError:
                continue
            except asyncio.CancelledError:
                break
