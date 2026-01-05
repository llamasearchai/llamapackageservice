"""
Retry utilities for handling transient failures.
"""

import asyncio
import functools
import logging
from dataclasses import dataclass
from typing import Callable, TypeVar, Any, Optional, Tuple, Type
import random

from ..error import ProcessorError

logger = logging.getLogger(__name__)

T = TypeVar('T')


@dataclass
class RetryConfig:
    """Configuration for retry behavior."""
    
    max_retries: int = 3
    base_delay: float = 1.0
    max_delay: float = 60.0
    exponential_base: float = 2.0
    jitter: bool = True
    retryable_exceptions: Tuple[Type[Exception], ...] = (
        ConnectionError,
        TimeoutError,
        OSError,
    )


def calculate_delay(
    attempt: int,
    config: RetryConfig,
) -> float:
    """
    Calculate the delay before the next retry attempt.
    
    Uses exponential backoff with optional jitter.
    
    Args:
        attempt: The current attempt number (0-indexed)
        config: The retry configuration
        
    Returns:
        Delay in seconds
    """
    delay = config.base_delay * (config.exponential_base ** attempt)
    delay = min(delay, config.max_delay)
    
    if config.jitter:
        # Add random jitter between 0 and delay
        delay = delay * (0.5 + random.random())
    
    return delay


async def retry_async(
    func: Callable[..., Any],
    *args,
    config: Optional[RetryConfig] = None,
    **kwargs,
) -> T:
    """
    Retry an async function with exponential backoff.
    
    Args:
        func: The async function to retry
        *args: Positional arguments to pass to the function
        config: Retry configuration (uses defaults if not provided)
        **kwargs: Keyword arguments to pass to the function
        
    Returns:
        The result of the function
        
    Raises:
        The last exception if all retries fail
    """
    if config is None:
        config = RetryConfig()
    
    last_exception: Optional[Exception] = None
    
    for attempt in range(config.max_retries + 1):
        try:
            return await func(*args, **kwargs)
        except config.retryable_exceptions as e:
            last_exception = e
            
            if attempt < config.max_retries:
                delay = calculate_delay(attempt, config)
                logger.warning(
                    f"Attempt {attempt + 1} failed with {type(e).__name__}: {e}. "
                    f"Retrying in {delay:.2f}s..."
                )
                await asyncio.sleep(delay)
            else:
                logger.error(
                    f"All {config.max_retries + 1} attempts failed. "
                    f"Last error: {type(e).__name__}: {e}"
                )
        except Exception as e:
            # Non-retryable exception
            logger.error(f"Non-retryable error: {type(e).__name__}: {e}")
            raise
    
    # All retries exhausted
    if last_exception:
        raise last_exception
    raise RuntimeError("Retry failed with no exception")


def with_retry(
    config: Optional[RetryConfig] = None,
):
    """
    Decorator to add retry behavior to an async function.
    
    Args:
        config: Retry configuration (uses defaults if not provided)
        
    Example:
        @with_retry(RetryConfig(max_retries=5))
        async def fetch_data(url):
            ...
    """
    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        @functools.wraps(func)
        async def wrapper(*args, **kwargs):
            return await retry_async(func, *args, config=config, **kwargs)
        return wrapper
    return decorator


class CircuitBreaker:
    """
    Circuit breaker pattern implementation for fault tolerance.
    
    States:
    - CLOSED: Normal operation, requests go through
    - OPEN: Failing, requests are rejected immediately
    - HALF_OPEN: Testing, some requests go through
    """
    
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half_open"
    
    def __init__(
        self,
        failure_threshold: int = 5,
        recovery_timeout: float = 30.0,
        half_open_requests: int = 3,
    ):
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.half_open_requests = half_open_requests
        
        self._state = self.CLOSED
        self._failure_count = 0
        self._last_failure_time: Optional[float] = None
        self._half_open_successes = 0
        self._lock = asyncio.Lock()
    
    @property
    def state(self) -> str:
        """Get the current circuit breaker state."""
        return self._state
    
    async def can_execute(self) -> bool:
        """Check if a request can be executed."""
        async with self._lock:
            if self._state == self.CLOSED:
                return True
            
            if self._state == self.OPEN:
                # Check if recovery timeout has passed
                if self._last_failure_time is not None:
                    import time
                    elapsed = time.time() - self._last_failure_time
                    if elapsed >= self.recovery_timeout:
                        self._state = self.HALF_OPEN
                        self._half_open_successes = 0
                        logger.info("Circuit breaker entering half-open state")
                        return True
                return False
            
            # HALF_OPEN state
            return True
    
    async def record_success(self) -> None:
        """Record a successful request."""
        async with self._lock:
            if self._state == self.HALF_OPEN:
                self._half_open_successes += 1
                if self._half_open_successes >= self.half_open_requests:
                    self._state = self.CLOSED
                    self._failure_count = 0
                    logger.info("Circuit breaker closed")
            elif self._state == self.CLOSED:
                self._failure_count = 0
    
    async def record_failure(self) -> None:
        """Record a failed request."""
        async with self._lock:
            import time
            self._last_failure_time = time.time()
            
            if self._state == self.HALF_OPEN:
                self._state = self.OPEN
                logger.warning("Circuit breaker reopened after failure in half-open state")
            elif self._state == self.CLOSED:
                self._failure_count += 1
                if self._failure_count >= self.failure_threshold:
                    self._state = self.OPEN
                    logger.warning(
                        f"Circuit breaker opened after {self._failure_count} failures"
                    )
    
    async def execute(self, func: Callable[..., Any], *args, **kwargs) -> Any:
        """
        Execute a function with circuit breaker protection.
        
        Args:
            func: The function to execute
            *args: Positional arguments
            **kwargs: Keyword arguments
            
        Returns:
            The function result
            
        Raises:
            ProcessorError: If circuit is open
        """
        if not await self.can_execute():
            raise ProcessorError(
                f"Circuit breaker is open, request rejected. "
                f"Will retry in {self.recovery_timeout}s"
            )
        
        try:
            result = await func(*args, **kwargs)
            await self.record_success()
            return result
        except Exception as e:
            await self.record_failure()
            raise
