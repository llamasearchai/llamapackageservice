"""
Analysis types and data structures for AI-powered code analysis.
"""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Dict, Optional
import uuid


class AnalysisType(Enum):
    """Types of analysis that can be performed."""
    
    DOCUMENTATION = "documentation"
    """Generate comprehensive repository documentation."""
    
    CODE_REVIEW = "code_review"
    """Analyze code quality and suggest improvements."""
    
    API_DOCUMENTATION = "api_documentation"
    """Generate API documentation."""
    
    SECURITY_AUDIT = "security_audit"
    """Analyze dependencies and security."""
    
    EXAMPLES = "examples"
    """Generate usage examples."""
    
    CUSTOM = "custom"
    """Custom analysis with user-defined prompt."""


@dataclass
class AnalysisRequest:
    """Repository analysis request."""
    
    repository: str
    """Repository URL or path."""
    
    analysis_type: AnalysisType = AnalysisType.DOCUMENTATION
    """Type of analysis to perform."""
    
    context: Optional[str] = None
    """Additional context or instructions."""
    
    parameters: Dict[str, str] = field(default_factory=dict)
    """Custom parameters for the analysis."""


@dataclass
class AnalysisResult:
    """Analysis result from OpenAI agent."""
    
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    """Unique identifier for the analysis."""
    
    analysis_type: AnalysisType = AnalysisType.DOCUMENTATION
    """Type of analysis performed."""
    
    content: str = ""
    """Generated content."""
    
    confidence: float = 0.0
    """Confidence score (0.0 to 1.0)."""
    
    metadata: Dict[str, str] = field(default_factory=dict)
    """Metadata about the analysis."""
    
    timestamp: datetime = field(default_factory=datetime.utcnow)
    """Timestamp of analysis."""


@dataclass
class ConversationContext:
    """Conversation context for interactive analysis."""
    
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    """Conversation ID."""
    
    messages: list = field(default_factory=list)
    """Message history."""
    
    repository_context: str = ""
    """Repository context."""
    
    session_state: Dict[str, str] = field(default_factory=dict)
    """Analysis session state."""


@dataclass
class Message:
    """Individual message in a conversation."""
    
    role: str
    """Message role (user, assistant, system)."""
    
    content: str
    """Message content."""
    
    timestamp: datetime = field(default_factory=datetime.utcnow)
    """Message timestamp."""
