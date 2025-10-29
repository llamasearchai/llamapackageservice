"""Operate Enhanced - AI-powered computer automation framework."""
__version__ = "2.0.0"

from .interfaces import (
    Action,
    ActionType,
    OperationStatus,
    OperationResult,
)
from .core.orchestrator import OperationOrchestrator, Operation
from .security.guardian import SecurityGuardian
from .state.manager import StateManager
from .integrations.github_manager import GitHubManager

__all__ = [
    "Action",
    "ActionType",
    "OperationStatus",
    "OperationResult",
    "OperationOrchestrator",
    "Operation",
    "SecurityGuardian",
    "StateManager",
    "GitHubManager",
]