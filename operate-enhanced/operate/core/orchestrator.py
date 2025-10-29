"""Operation orchestrator with advanced error handling and recovery."""
import asyncio
import uuid
from collections import deque
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, Deque, Dict, List, Optional, Set
import logging

from ..interfaces import (
    Action,
    ActionType,
    OperationResult,
    OperationStatus,
    IActionInterface,
    IStateManager,
    ISecurityValidator,
)


logger = logging.getLogger(__name__)


@dataclass
class Operation:
    """Enhanced operation with validation and rollback."""
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    action: Action = None
    pre_validation: Optional[Callable] = None
    post_validation: Optional[Callable] = None
    rollback: Optional[Callable] = None
    dependencies: Set[str] = field(default_factory=set)
    metadata: Dict[str, Any] = field(default_factory=dict)
    retry_count: int = 0
    max_retries: int = 3
    timeout: Optional[float] = None


@dataclass
class ExecutionContext:
    """Context for operation execution."""
    operation: Operation
    start_time: datetime
    checkpoint_id: Optional[str] = None
    parent_context: Optional["ExecutionContext"] = None
    child_contexts: List["ExecutionContext"] = field(default_factory=list)


class OperationOrchestrator:
    """Orchestrates operation execution with advanced features."""
    
    def __init__(
        self,
        action_interface: IActionInterface,
        state_manager: IStateManager,
        security_validator: ISecurityValidator,
        max_parallel: int = 4,
        enable_learning: bool = True,
    ):
        self.action_interface = action_interface
        self.state_manager = state_manager
        self.security_validator = security_validator
        self.max_parallel = max_parallel
        self.enable_learning = enable_learning
        
        self.operation_queue: Deque[Operation] = deque()
        self.executing: Dict[str, ExecutionContext] = {}
        self.completed: List[OperationResult] = []
        self.patterns: List[Dict[str, Any]] = []
        self._lock = asyncio.Lock()
        
    async def execute_operation(
        self,
        operation: Operation,
        parent_context: Optional[ExecutionContext] = None
    ) -> OperationResult:
        """Execute a single operation with full error handling."""
        context = ExecutionContext(
            operation=operation,
            start_time=datetime.utcnow(),
            parent_context=parent_context
        )
        
        try:
            # Security validation
            is_allowed, reason = await self.security_validator.validate_action(operation.action)
            if not is_allowed:
                logger.warning(f"Operation {operation.id} blocked: {reason}")
                return OperationResult(
                    operation_id=operation.id,
                    status=OperationStatus.FAILED,
                    error=PermissionError(reason)
                )
            
            # Pre-validation
            if operation.pre_validation:
                if not await operation.pre_validation():
                    raise ValueError(f"Pre-validation failed for operation {operation.id}")
            
            # Create checkpoint
            if operation.rollback:
                context.checkpoint_id = await self.state_manager.create_checkpoint()
            
            # Execute with timeout
            if operation.timeout:
                result = await asyncio.wait_for(
                    self._execute_action(operation, context),
                    timeout=operation.timeout
                )
            else:
                result = await self._execute_action(operation, context)
            
            # Post-validation
            if operation.post_validation:
                if not await operation.post_validation(result):
                    raise RuntimeError(f"Post-validation failed for operation {operation.id}")
            
            # Learn from success
            if self.enable_learning and result.status == OperationStatus.SUCCESS:
                await self._learn_from_execution(operation, result)
            
            return result
            
        except asyncio.TimeoutError:
            logger.error(f"Operation {operation.id} timed out")
            return await self._handle_failure(operation, context, TimeoutError("Operation timed out"))
            
        except Exception as e:
            logger.error(f"Operation {operation.id} failed: {str(e)}")
            return await self._handle_failure(operation, context, e)
            
    async def _execute_action(self, operation: Operation, context: ExecutionContext) -> OperationResult:
        """Execute the actual action."""
        async with self._lock:
            self.executing[operation.id] = context
        
        try:
            result = await self.action_interface.execute_action(operation.action)
            result.duration = (datetime.utcnow() - context.start_time).total_seconds()
            return result
        finally:
            async with self._lock:
                del self.executing[operation.id]
                
    async def _handle_failure(
        self,
        operation: Operation,
        context: ExecutionContext,
        error: Exception
    ) -> OperationResult:
        """Handle operation failure with retry and rollback."""
        # Attempt retry
        if operation.retry_count < operation.max_retries:
            operation.retry_count += 1
            logger.info(f"Retrying operation {operation.id} (attempt {operation.retry_count})")
            await asyncio.sleep(2 ** operation.retry_count)  # Exponential backoff
            return await self.execute_operation(operation, context.parent_context)
        
        # Rollback if available
        if operation.rollback and context.checkpoint_id:
            try:
                await operation.rollback()
                await self.state_manager.restore_checkpoint(context.checkpoint_id)
                return OperationResult(
                    operation_id=operation.id,
                    status=OperationStatus.ROLLED_BACK,
                    error=error
                )
            except Exception as rollback_error:
                logger.error(f"Rollback failed for operation {operation.id}: {str(rollback_error)}")
        
        return OperationResult(
            operation_id=operation.id,
            status=OperationStatus.FAILED,
            error=error
        )
    
    async def execute_sequence(self, operations: List[Operation]) -> List[OperationResult]:
        """Execute a sequence of operations."""
        results = []
        for operation in operations:
            result = await self.execute_operation(operation)
            results.append(result)
            
            # Stop on failure unless explicitly configured to continue
            if result.status == OperationStatus.FAILED and not operation.metadata.get("continue_on_failure"):
                break
                
        return results
    
    async def execute_parallel(self, operations: List[Operation]) -> List[OperationResult]:
        """Execute operations in parallel with dependency resolution."""
        # Build dependency graph
        dependency_graph = self._build_dependency_graph(operations)
        
        # Execute in waves based on dependencies
        results = []
        while dependency_graph:
            # Find operations with no dependencies
            ready = [
                op for op in dependency_graph
                if not dependency_graph[op]
            ]
            
            if not ready:
                raise RuntimeError("Circular dependency detected in operations")
            
            # Execute ready operations in parallel
            tasks = [
                self.execute_operation(op)
                for op in ready[:self.max_parallel]
            ]
            
            wave_results = await asyncio.gather(*tasks, return_exceptions=True)
            
            # Process results and update dependencies
            for op, result in zip(ready[:self.max_parallel], wave_results):
                if isinstance(result, Exception):
                    result = OperationResult(
                        operation_id=op.id,
                        status=OperationStatus.FAILED,
                        error=result
                    )
                
                results.append(result)
                
                # Remove from graph
                del dependency_graph[op]
                
                # Update dependencies
                for other_op in dependency_graph:
                    dependency_graph[other_op].discard(op.id)
        
        return results
    
    def _build_dependency_graph(self, operations: List[Operation]) -> Dict[Operation, Set[str]]:
        """Build dependency graph for operations."""
        op_map = {op.id: op for op in operations}
        return {
            op: op.dependencies.copy()
            for op in operations
        }
    
    async def _learn_from_execution(self, operation: Operation, result: OperationResult):
        """Learn patterns from successful executions."""
        pattern = {
            "action_type": operation.action.type,
            "metadata": operation.action.metadata,
            "duration": result.duration,
            "timestamp": result.timestamp,
        }
        
        self.patterns.append(pattern)
        
        # Save patterns periodically
        if len(self.patterns) % 10 == 0:
            await self.state_manager.save_state("execution_patterns", self.patterns)
    
    async def suggest_optimizations(self, operations: List[Operation]) -> List[Dict[str, Any]]:
        """Suggest optimizations based on learned patterns."""
        suggestions = []
        
        # Analyze for parallelization opportunities
        for i, op in enumerate(operations[:-1]):
            next_op = operations[i + 1]
            if not next_op.dependencies and self._can_parallelize(op, next_op):
                suggestions.append({
                    "type": "parallelize",
                    "operations": [op.id, next_op.id],
                    "reason": "No dependencies between operations"
                })
        
        # Suggest caching for repeated operations
        seen = {}
        for op in operations:
            key = (op.action.type, op.action.target)
            if key in seen:
                suggestions.append({
                    "type": "cache",
                    "operation": op.id,
                    "similar_to": seen[key],
                    "reason": "Duplicate operation detected"
                })
            else:
                seen[key] = op.id
        
        return suggestions
    
    def _can_parallelize(self, op1: Operation, op2: Operation) -> bool:
        """Check if two operations can be parallelized."""
        # Don't parallelize operations on same target
        if op1.action.target == op2.action.target:
            return False
        
        # Don't parallelize certain action types
        sequential_types = {ActionType.TYPE, ActionType.EXECUTE}
        if op1.action.type in sequential_types or op2.action.type in sequential_types:
            return False
        
        return True