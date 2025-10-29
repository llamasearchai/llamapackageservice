"""State management and persistence system."""
import asyncio
import json
import pickle
import uuid
from collections import defaultdict
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple, Union
import logging

import aiofiles
import redis.asyncio as redis
from pydantic import BaseModel

from ..interfaces import IStateManager, Action, OperationResult


logger = logging.getLogger(__name__)


class StorageBackend(BaseModel):
    """Storage backend configuration."""
    type: str = "file"  # file, redis, memory
    path: Optional[str] = "./state"
    redis_url: Optional[str] = None
    ttl: int = 3600  # Default TTL in seconds


@dataclass
class StateSnapshot:
    """Represents a state snapshot."""
    id: str
    timestamp: datetime
    data: Dict[str, Any]
    metadata: Dict[str, Any]
    parent_id: Optional[str] = None


@dataclass
class StateTransaction:
    """Represents a state transaction."""
    id: str
    operations: List[Tuple[str, str, Any]]  # (operation, key, value)
    timestamp: datetime
    committed: bool = False


class StateStore:
    """Abstract base for state storage backends."""
    
    async def get(self, key: str) -> Optional[Any]:
        raise NotImplementedError
        
    async def set(self, key: str, value: Any, ttl: Optional[int] = None):
        raise NotImplementedError
        
    async def delete(self, key: str):
        raise NotImplementedError
        
    async def exists(self, key: str) -> bool:
        raise NotImplementedError
        
    async def keys(self, pattern: str = "*") -> List[str]:
        raise NotImplementedError


class FileStateStore(StateStore):
    """File-based state storage."""
    
    def __init__(self, base_path: str):
        self.base_path = Path(base_path)
        self.base_path.mkdir(parents=True, exist_ok=True)
        self._locks: Dict[str, asyncio.Lock] = defaultdict(asyncio.Lock)
        
    def _get_path(self, key: str) -> Path:
        # Create subdirectories based on key structure
        parts = key.split('/')
        if len(parts) > 1:
            subdir = self.base_path / '/'.join(parts[:-1])
            subdir.mkdir(parents=True, exist_ok=True)
            return subdir / f"{parts[-1]}.json"
        return self.base_path / f"{key}.json"
        
    async def get(self, key: str) -> Optional[Any]:
        path = self._get_path(key)
        if not path.exists():
            return None
            
        async with self._locks[key]:
            try:
                async with aiofiles.open(path, 'r') as f:
                    data = json.loads(await f.read())
                    
                # Check TTL
                if 'ttl' in data and 'timestamp' in data:
                    timestamp = datetime.fromisoformat(data['timestamp'])
                    if datetime.utcnow() - timestamp > timedelta(seconds=data['ttl']):
                        await self.delete(key)
                        return None
                        
                return data.get('value')
            except Exception as e:
                logger.error(f"Error reading state {key}: {str(e)}")
                return None
                
    async def set(self, key: str, value: Any, ttl: Optional[int] = None):
        path = self._get_path(key)
        
        async with self._locks[key]:
            data = {
                'value': value,
                'timestamp': datetime.utcnow().isoformat()
            }
            if ttl:
                data['ttl'] = ttl
                
            try:
                async with aiofiles.open(path, 'w') as f:
                    await f.write(json.dumps(data, default=str))
            except Exception as e:
                logger.error(f"Error writing state {key}: {str(e)}")
                raise
                
    async def delete(self, key: str):
        path = self._get_path(key)
        if path.exists():
            path.unlink()
            
    async def exists(self, key: str) -> bool:
        path = self._get_path(key)
        return path.exists()
        
    async def keys(self, pattern: str = "*") -> List[str]:
        import fnmatch
        keys = []
        
        for path in self.base_path.rglob("*.json"):
            key = str(path.relative_to(self.base_path))[:-5]  # Remove .json
            if fnmatch.fnmatch(key, pattern):
                keys.append(key)
                
        return keys


class RedisStateStore(StateStore):
    """Redis-based state storage."""
    
    def __init__(self, redis_url: str):
        self.redis_url = redis_url
        self._client: Optional[redis.Redis] = None
        
    async def _get_client(self) -> redis.Redis:
        if not self._client:
            self._client = await redis.from_url(self.redis_url)
        return self._client
        
    async def get(self, key: str) -> Optional[Any]:
        client = await self._get_client()
        value = await client.get(key)
        if value:
            try:
                return json.loads(value)
            except:
                return pickle.loads(value)
        return None
        
    async def set(self, key: str, value: Any, ttl: Optional[int] = None):
        client = await self._get_client()
        try:
            serialized = json.dumps(value, default=str)
        except:
            serialized = pickle.dumps(value)
            
        if ttl:
            await client.setex(key, ttl, serialized)
        else:
            await client.set(key, serialized)
            
    async def delete(self, key: str):
        client = await self._get_client()
        await client.delete(key)
        
    async def exists(self, key: str) -> bool:
        client = await self._get_client()
        return await client.exists(key)
        
    async def keys(self, pattern: str = "*") -> List[str]:
        client = await self._get_client()
        keys = await client.keys(pattern)
        return [k.decode() for k in keys]


class MemoryStateStore(StateStore):
    """In-memory state storage."""
    
    def __init__(self):
        self._data: Dict[str, Tuple[Any, Optional[datetime]]] = {}
        self._lock = asyncio.Lock()
        
    async def get(self, key: str) -> Optional[Any]:
        async with self._lock:
            if key not in self._data:
                return None
                
            value, expiry = self._data[key]
            if expiry and datetime.utcnow() > expiry:
                del self._data[key]
                return None
                
            return value
            
    async def set(self, key: str, value: Any, ttl: Optional[int] = None):
        async with self._lock:
            expiry = None
            if ttl:
                expiry = datetime.utcnow() + timedelta(seconds=ttl)
            self._data[key] = (value, expiry)
            
    async def delete(self, key: str):
        async with self._lock:
            self._data.pop(key, None)
            
    async def exists(self, key: str) -> bool:
        return key in self._data
        
    async def keys(self, pattern: str = "*") -> List[str]:
        import fnmatch
        async with self._lock:
            return [k for k in self._data.keys() if fnmatch.fnmatch(k, pattern)]


class StateManager(IStateManager):
    """Main state management system with transactions and snapshots."""
    
    def __init__(self, backend: StorageBackend):
        self.backend = backend
        self._store = self._create_store()
        self._snapshots: Dict[str, StateSnapshot] = {}
        self._transactions: Dict[str, StateTransaction] = {}
        self._current_transaction: Optional[StateTransaction] = None
        
    def _create_store(self) -> StateStore:
        """Create the appropriate storage backend."""
        if self.backend.type == "file":
            return FileStateStore(self.backend.path or "./state")
        elif self.backend.type == "redis":
            if not self.backend.redis_url:
                raise ValueError("Redis URL required for redis backend")
            return RedisStateStore(self.backend.redis_url)
        elif self.backend.type == "memory":
            return MemoryStateStore()
        else:
            raise ValueError(f"Unknown backend type: {self.backend.type}")
            
    async def save_state(self, key: str, value: Any) -> None:
        """Save state value."""
        # If in transaction, queue the operation
        if self._current_transaction:
            self._current_transaction.operations.append(("set", key, value))
        else:
            await self._store.set(key, value, self.backend.ttl)
            
    async def load_state(self, key: str) -> Optional[Any]:
        """Load state value."""
        return await self._store.get(key)
        
    async def delete_state(self, key: str) -> None:
        """Delete state value."""
        if self._current_transaction:
            self._current_transaction.operations.append(("delete", key, None))
        else:
            await self._store.delete(key)
            
    async def create_checkpoint(self) -> str:
        """Create a state checkpoint."""
        checkpoint_id = str(uuid.uuid4())
        
        # Capture current state
        all_keys = await self._store.keys()
        data = {}
        for key in all_keys:
            value = await self._store.get(key)
            if value is not None:
                data[key] = value
                
        snapshot = StateSnapshot(
            id=checkpoint_id,
            timestamp=datetime.utcnow(),
            data=data,
            metadata={
                "key_count": len(data),
                "size_estimate": len(json.dumps(data, default=str))
            }
        )
        
        self._snapshots[checkpoint_id] = snapshot
        
        # Persist snapshot
        await self._store.set(f"_snapshots/{checkpoint_id}", asdict(snapshot))
        
        logger.info(f"Created checkpoint {checkpoint_id} with {len(data)} keys")
        return checkpoint_id
        
    async def restore_checkpoint(self, checkpoint_id: str) -> None:
        """Restore from checkpoint."""
        # Load snapshot
        if checkpoint_id in self._snapshots:
            snapshot = self._snapshots[checkpoint_id]
        else:
            snapshot_data = await self._store.get(f"_snapshots/{checkpoint_id}")
            if not snapshot_data:
                raise ValueError(f"Checkpoint {checkpoint_id} not found")
            snapshot = StateSnapshot(**snapshot_data)
            
        # Clear current state
        current_keys = await self._store.keys()
        for key in current_keys:
            if not key.startswith("_snapshots/"):
                await self._store.delete(key)
                
        # Restore snapshot data
        for key, value in snapshot.data.items():
            await self._store.set(key, value)
            
        logger.info(f"Restored checkpoint {checkpoint_id} with {len(snapshot.data)} keys")
        
    async def begin_transaction(self) -> str:
        """Begin a new transaction."""
        if self._current_transaction:
            raise RuntimeError("Transaction already in progress")
            
        transaction_id = str(uuid.uuid4())
        self._current_transaction = StateTransaction(
            id=transaction_id,
            operations=[],
            timestamp=datetime.utcnow()
        )
        
        self._transactions[transaction_id] = self._current_transaction
        logger.info(f"Started transaction {transaction_id}")
        return transaction_id
        
    async def commit_transaction(self, transaction_id: str) -> None:
        """Commit a transaction."""
        if not self._current_transaction or self._current_transaction.id != transaction_id:
            raise ValueError(f"Invalid transaction {transaction_id}")
            
        # Apply all operations
        for op_type, key, value in self._current_transaction.operations:
            if op_type == "set":
                await self._store.set(key, value, self.backend.ttl)
            elif op_type == "delete":
                await self._store.delete(key)
                
        self._current_transaction.committed = True
        self._current_transaction = None
        logger.info(f"Committed transaction {transaction_id}")
        
    async def rollback_transaction(self, transaction_id: str) -> None:
        """Rollback a transaction."""
        if not self._current_transaction or self._current_transaction.id != transaction_id:
            raise ValueError(f"Invalid transaction {transaction_id}")
            
        # Discard all operations
        self._current_transaction = None
        logger.info(f"Rolled back transaction {transaction_id}")
        
    async def save_operation_history(self, operations: List[OperationResult]) -> None:
        """Save operation history."""
        history_key = f"history/{datetime.utcnow().strftime('%Y%m%d_%H%M%S')}"
        history_data = [
            {
                "operation_id": op.operation_id,
                "status": op.status.value,
                "timestamp": op.timestamp.isoformat(),
                "duration": op.duration,
                "error": str(op.error) if op.error else None
            }
            for op in operations
        ]
        
        await self.save_state(history_key, history_data)
        
    async def get_operation_history(self, limit: int = 100) -> List[Dict[str, Any]]:
        """Get operation history."""
        history_keys = await self._store.keys("history/*")
        history_keys.sort(reverse=True)  # Most recent first
        
        all_history = []
        for key in history_keys[:limit]:
            data = await self.load_state(key)
            if data:
                all_history.extend(data)
                
        return all_history[:limit]
        
    async def save_pattern(self, pattern_name: str, actions: List[Action]) -> None:
        """Save an action pattern for reuse."""
        pattern_data = [
            {
                "id": action.id,
                "type": action.type.value,
                "target": str(action.target) if action.target else None,
                "value": action.value,
                "metadata": action.metadata
            }
            for action in actions
        ]
        
        await self.save_state(f"patterns/{pattern_name}", pattern_data)
        
    async def load_pattern(self, pattern_name: str) -> Optional[List[Action]]:
        """Load a saved action pattern."""
        pattern_data = await self.load_state(f"patterns/{pattern_name}")
        if not pattern_data:
            return None
            
        from ..interfaces import ActionType, Coordinate
        
        actions = []
        for data in pattern_data:
            action = Action(
                id=data["id"],
                type=ActionType(data["type"]),
                target=data.get("target"),
                value=data.get("value"),
                metadata=data.get("metadata", {})
            )
            actions.append(action)
            
        return actions
        
    async def get_metrics(self) -> Dict[str, Any]:
        """Get state management metrics."""
        all_keys = await self._store.keys()
        
        metrics = {
            "total_keys": len(all_keys),
            "checkpoint_count": len([k for k in all_keys if k.startswith("_snapshots/")]),
            "pattern_count": len([k for k in all_keys if k.startswith("patterns/")]),
            "history_entries": len([k for k in all_keys if k.startswith("history/")]),
            "active_transaction": self._current_transaction is not None,
            "backend_type": self.backend.type
        }
        
        return metrics
        
    async def cleanup_old_data(self, days: int = 7) -> int:
        """Clean up old data."""
        cutoff = datetime.utcnow() - timedelta(days=days)
        deleted = 0
        
        # Clean old history
        history_keys = await self._store.keys("history/*")
        for key in history_keys:
            # Extract date from key
            date_str = key.split('/')[-1].split('_')[0]
            try:
                key_date = datetime.strptime(date_str, '%Y%m%d')
                if key_date < cutoff:
                    await self._store.delete(key)
                    deleted += 1
            except:
                pass
                
        # Clean old snapshots
        snapshot_keys = await self._store.keys("_snapshots/*")
        for key in snapshot_keys:
            snapshot_data = await self._store.get(key)
            if snapshot_data and 'timestamp' in snapshot_data:
                timestamp = datetime.fromisoformat(snapshot_data['timestamp'])
                if timestamp < cutoff:
                    await self._store.delete(key)
                    deleted += 1
                    
        logger.info(f"Cleaned up {deleted} old entries")
        return deleted