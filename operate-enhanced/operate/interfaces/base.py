"""Core interfaces for the enhanced operate framework."""
from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple, Callable, Union
import asyncio
from datetime import datetime


class ActionType(Enum):
    """Types of actions that can be performed."""
    CLICK = "click"
    TYPE = "type"
    KEY = "key"
    SCREENSHOT = "screenshot"
    WAIT = "wait"
    SCROLL = "scroll"
    DRAG = "drag"
    HOVER = "hover"
    EXECUTE = "execute"
    GITHUB = "github"


class OperationStatus(Enum):
    """Status of an operation."""
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
    CANCELLED = "cancelled"
    ROLLED_BACK = "rolled_back"


@dataclass
class Coordinate:
    """Represents a screen coordinate."""
    x: int
    y: int
    
    @classmethod
    def from_percentage(cls, x_percent: float, y_percent: float, screen_width: int, screen_height: int) -> "Coordinate":
        """Create coordinate from percentage values."""
        return cls(
            x=int(x_percent * screen_width / 100),
            y=int(y_percent * screen_height / 100)
        )


@dataclass
class Action:
    """Represents an action to be performed."""
    id: str
    type: ActionType
    target: Optional[Union[Coordinate, str]] = None
    value: Optional[Any] = None
    metadata: Dict[str, Any] = None
    
    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}


@dataclass
class OperationResult:
    """Result of an operation execution."""
    operation_id: str
    status: OperationStatus
    result: Optional[Any] = None
    error: Optional[Exception] = None
    timestamp: datetime = None
    duration: Optional[float] = None
    
    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = datetime.utcnow()


class IScreenInterface(ABC):
    """Interface for screen operations."""
    
    @abstractmethod
    async def capture_screenshot(self) -> bytes:
        """Capture the current screen."""
        pass
    
    @abstractmethod
    async def get_screen_dimensions(self) -> Tuple[int, int]:
        """Get screen width and height."""
        pass
    
    @abstractmethod
    async def find_element(self, identifier: str) -> Optional[Coordinate]:
        """Find an element on screen by identifier."""
        pass


class IActionInterface(ABC):
    """Interface for performing actions."""
    
    @abstractmethod
    async def execute_action(self, action: Action) -> OperationResult:
        """Execute a single action."""
        pass
    
    @abstractmethod
    async def validate_action(self, action: Action) -> bool:
        """Validate if an action can be performed."""
        pass
    
    @abstractmethod
    async def get_supported_actions(self) -> List[ActionType]:
        """Get list of supported action types."""
        pass


class IModelInterface(ABC):
    """Interface for AI model interactions."""
    
    @abstractmethod
    async def analyze_screen(self, screenshot: bytes, objective: str) -> Action:
        """Analyze screenshot and determine next action."""
        pass
    
    @abstractmethod
    async def decompose_task(self, task: str) -> List[str]:
        """Break down a complex task into subtasks."""
        pass
    
    @abstractmethod
    async def validate_result(self, expected: str, actual: Any) -> bool:
        """Validate if result matches expectation."""
        pass


class IPlatformInterface(ABC):
    """Interface for platform-specific operations."""
    
    @abstractmethod
    def get_platform_name(self) -> str:
        """Get the platform name."""
        pass
    
    @abstractmethod
    async def execute_system_command(self, command: str) -> str:
        """Execute a system command."""
        pass
    
    @abstractmethod
    def get_accessibility_api(self) -> Optional[Any]:
        """Get platform-specific accessibility API."""
        pass


class IStateManager(ABC):
    """Interface for state management."""
    
    @abstractmethod
    async def save_state(self, key: str, value: Any) -> None:
        """Save state value."""
        pass
    
    @abstractmethod
    async def load_state(self, key: str) -> Optional[Any]:
        """Load state value."""
        pass
    
    @abstractmethod
    async def create_checkpoint(self) -> str:
        """Create a state checkpoint."""
        pass
    
    @abstractmethod
    async def restore_checkpoint(self, checkpoint_id: str) -> None:
        """Restore from checkpoint."""
        pass


class ISecurityValidator(ABC):
    """Interface for security validation."""
    
    @abstractmethod
    async def validate_action(self, action: Action) -> Tuple[bool, Optional[str]]:
        """Validate action against security rules."""
        pass
    
    @abstractmethod
    async def is_sandbox_mode(self) -> bool:
        """Check if running in sandbox mode."""
        pass
    
    @abstractmethod
    async def request_permission(self, action: Action) -> bool:
        """Request user permission for action."""
        pass


class IPlugin(ABC):
    """Interface for plugins."""
    
    @abstractmethod
    def get_name(self) -> str:
        """Get plugin name."""
        pass
    
    @abstractmethod
    def get_version(self) -> str:
        """Get plugin version."""
        pass
    
    @abstractmethod
    async def initialize(self, context: Dict[str, Any]) -> None:
        """Initialize the plugin."""
        pass
    
    @abstractmethod
    async def execute(self, action: Action) -> OperationResult:
        """Execute plugin action."""
        pass
    
    @abstractmethod
    def get_supported_actions(self) -> List[ActionType]:
        """Get actions supported by this plugin."""
        pass