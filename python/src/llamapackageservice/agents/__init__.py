"""
OpenAI Agents SDK Integration Module.

Provides integration with OpenAI's API for intelligent code analysis,
repository understanding, and automated documentation generation.
"""

from .openai_agent import OpenAIAgent, AgentConfig
from .analysis import AnalysisRequest, AnalysisResult, AnalysisType

__all__ = [
    "OpenAIAgent",
    "AgentConfig",
    "AnalysisRequest",
    "AnalysisResult",
    "AnalysisType",
]
