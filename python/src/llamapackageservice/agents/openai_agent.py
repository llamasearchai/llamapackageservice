"""
OpenAI Agent for code analysis.
"""

import os
from dataclasses import dataclass, field
from typing import Optional, Dict
from datetime import datetime
import logging
import uuid

from ..error import ProcessorError, LLMError
from .analysis import AnalysisRequest, AnalysisResult, AnalysisType

logger = logging.getLogger(__name__)


@dataclass
class AgentConfig:
    """Configuration for OpenAI agents."""
    
    api_key: str = ""
    """OpenAI API key."""
    
    model: str = "gpt-4"
    """Model to use for analysis."""
    
    max_tokens: int = 4000
    """Maximum tokens per request."""
    
    temperature: float = 0.7
    """Temperature for response generation."""
    
    system_prompt: str = ""
    """System prompt for repository analysis."""


class OpenAIAgent:
    """
    OpenAI Agents client wrapper.
    
    Provides AI-powered code analysis using OpenAI's API.
    """
    
    def __init__(self, config: AgentConfig):
        """
        Create a new OpenAI agent.
        
        Args:
            config: Agent configuration
        """
        self.config = config
        self._client = None
    
    @classmethod
    def from_env(cls) -> "OpenAIAgent":
        """
        Create agent from environment variables.
        
        Returns:
            Configured OpenAIAgent instance
            
        Raises:
            ProcessorError: If required environment variables are not set
        """
        api_key = os.getenv("OPENAI_API_KEY")
        if not api_key:
            raise ProcessorError("OPENAI_API_KEY environment variable not set")
        
        config = AgentConfig(
            api_key=api_key,
            model=os.getenv("OPENAI_MODEL", "gpt-4"),
            max_tokens=int(os.getenv("OPENAI_MAX_TOKENS", "4000")),
            temperature=float(os.getenv("OPENAI_TEMPERATURE", "0.7")),
            system_prompt=_get_default_system_prompt(),
        )
        
        return cls(config)
    
    async def _ensure_client(self):
        """Ensure the OpenAI client is initialized."""
        if self._client is None:
            try:
                from openai import AsyncOpenAI
                self._client = AsyncOpenAI(api_key=self.config.api_key)
            except ImportError:
                raise LLMError("OpenAI package not installed. Run: pip install openai")
        return self._client
    
    async def analyze_repository(self, request: AnalysisRequest) -> AnalysisResult:
        """
        Analyze a repository using OpenAI.
        
        Args:
            request: Analysis request with repository info and parameters
            
        Returns:
            Analysis result with generated content
        """
        try:
            client = await self._ensure_client()
            
            # Build the prompt based on analysis type
            prompt = self._build_prompt(request)
            
            # Make the API call
            response = await client.chat.completions.create(
                model=self.config.model,
                messages=[
                    {"role": "system", "content": self.config.system_prompt},
                    {"role": "user", "content": prompt},
                ],
                max_tokens=self.config.max_tokens,
                temperature=self.config.temperature,
            )
            
            content = response.choices[0].message.content or ""
            
            return AnalysisResult(
                id=str(uuid.uuid4()),
                analysis_type=request.analysis_type,
                content=content,
                confidence=0.9,  # Placeholder confidence
                metadata={
                    "model": self.config.model,
                    "repository": request.repository,
                },
                timestamp=datetime.utcnow(),
            )
            
        except Exception as e:
            logger.error(f"OpenAI analysis failed: {e}")
            # Return a fallback result
            return AnalysisResult(
                id=str(uuid.uuid4()),
                analysis_type=request.analysis_type,
                content=f"Analysis failed: {e}",
                confidence=0.0,
                metadata={"error": str(e)},
                timestamp=datetime.utcnow(),
            )
    
    def _build_prompt(self, request: AnalysisRequest) -> str:
        """Build the analysis prompt based on request type."""
        base = f"Analyze the repository: {request.repository}\n\n"
        
        type_prompts = {
            AnalysisType.DOCUMENTATION: (
                "Generate comprehensive documentation for this repository. "
                "Include an overview, installation instructions, usage examples, "
                "and API documentation."
            ),
            AnalysisType.CODE_REVIEW: (
                "Perform a code review of this repository. "
                "Identify potential issues, suggest improvements, "
                "and highlight good practices."
            ),
            AnalysisType.API_DOCUMENTATION: (
                "Generate detailed API documentation for this repository. "
                "Document all public functions, classes, and methods."
            ),
            AnalysisType.SECURITY_AUDIT: (
                "Perform a security audit of this repository. "
                "Identify potential vulnerabilities, insecure practices, "
                "and recommend security improvements."
            ),
            AnalysisType.EXAMPLES: (
                "Generate usage examples for this repository. "
                "Create clear, practical examples that demonstrate key features."
            ),
            AnalysisType.CUSTOM: request.context or "Analyze this repository.",
        }
        
        prompt = base + type_prompts.get(request.analysis_type, "Analyze this repository.")
        
        if request.context and request.analysis_type != AnalysisType.CUSTOM:
            prompt += f"\n\nAdditional context: {request.context}"
        
        return prompt


def _get_default_system_prompt() -> str:
    """Get the default system prompt for code analysis."""
    return """You are an expert software engineer and technical writer.
Your task is to analyze code repositories and generate high-quality documentation,
code reviews, and insights.

When analyzing code:
1. Be thorough but concise
2. Provide actionable recommendations
3. Use clear, professional language
4. Include code examples where appropriate
5. Consider best practices and industry standards

Format your responses in Markdown for readability."""
