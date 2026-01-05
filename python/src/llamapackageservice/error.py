"""
Error types for the package processing system.

This module provides error handling types and utilities for the LlamaPackageService.
"""

from typing import Optional
from enum import Enum


class ErrorKind(Enum):
    """Enumeration of error kinds for classification."""
    IO = "io"
    HTTP = "http"
    JSON = "json"
    ZIP = "zip"
    WALKDIR = "walkdir"
    URL_PARSE = "url_parse"
    DOWNLOAD = "download"
    MESSAGE = "message"
    NETWORK = "network"
    VALIDATION = "validation"
    PROCESSING = "processing"
    CONFIG = "config"
    RATE_LIMIT = "rate_limit"
    GITHUB_API = "github_api"
    LLM = "llm"
    PARSE = "parse"
    DATABASE = "database"
    OPENAI = "openai"
    PYPI_API = "pypi_api"
    NPM_API = "npm_api"
    CACHE = "cache"


class ProcessorError(Exception):
    """
    Base exception class for all processor errors.
    
    Attributes:
        message: The error message
        kind: The kind of error (for classification)
        source: Optional underlying exception that caused this error
    """
    
    def __init__(
        self,
        message: str,
        kind: ErrorKind = ErrorKind.MESSAGE,
        source: Optional[Exception] = None
    ):
        super().__init__(message)
        self.message = message
        self.kind = kind
        self.source = source
    
    def __str__(self) -> str:
        return self.message
    
    def __repr__(self) -> str:
        return f"ProcessorError(message={self.message!r}, kind={self.kind})"
    
    def is_transient(self) -> bool:
        """Check if this error is transient and retryable."""
        return self.kind in {
            ErrorKind.NETWORK,
            ErrorKind.HTTP,
            ErrorKind.RATE_LIMIT,
            ErrorKind.IO,
        }
    
    def is_fatal(self) -> bool:
        """Check if this error is fatal and should terminate processing."""
        return not self.is_transient()


class IOError(ProcessorError):
    """I/O errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"IO error: {message}", ErrorKind.IO, source)


class HttpError(ProcessorError):
    """HTTP request/response errors."""
    
    def __init__(self, message: str, status_code: Optional[int] = None, source: Optional[Exception] = None):
        self.status_code = status_code
        super().__init__(f"HTTP error: {message}", ErrorKind.HTTP, source)


class JsonError(ProcessorError):
    """JSON parsing/serialization errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"JSON error: {message}", ErrorKind.JSON, source)


class ValidationError(ProcessorError):
    """Input validation errors."""
    
    def __init__(self, message: str):
        super().__init__(f"Validation error: {message}", ErrorKind.VALIDATION)


class NetworkError(ProcessorError):
    """Network connectivity errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"Network error: {message}", ErrorKind.NETWORK, source)


class ProcessingError(ProcessorError):
    """Package processing errors."""
    
    def __init__(self, message: str):
        super().__init__(f"Processing error: {message}", ErrorKind.PROCESSING)


class ConfigError(ProcessorError):
    """Configuration errors."""
    
    def __init__(self, message: str):
        super().__init__(f"Config error: {message}", ErrorKind.CONFIG)


class RateLimitError(ProcessorError):
    """API rate limit exceeded errors."""
    
    def __init__(self, message: str, retry_after: Optional[int] = None):
        self.retry_after = retry_after
        super().__init__(f"Rate limit exceeded: {message}", ErrorKind.RATE_LIMIT)


class GitHubApiError(ProcessorError):
    """GitHub API specific errors."""
    
    def __init__(self, message: str, status_code: Optional[int] = None):
        self.status_code = status_code
        super().__init__(f"GitHub API error: {message}", ErrorKind.GITHUB_API)


class LLMError(ProcessorError):
    """Language Model errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"LLM error: {message}", ErrorKind.LLM, source)


class ParseError(ProcessorError):
    """Parsing errors."""
    
    def __init__(self, message: str):
        super().__init__(f"Parsing error: {message}", ErrorKind.PARSE)


class DatabaseError(ProcessorError):
    """Database errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"Database error: {message}", ErrorKind.DATABASE, source)


class OpenAIError(ProcessorError):
    """OpenAI API errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"OpenAI error: {message}", ErrorKind.OPENAI, source)


class PyPiApiError(ProcessorError):
    """PyPI API errors."""
    
    def __init__(self, message: str, status_code: Optional[int] = None):
        self.status_code = status_code
        super().__init__(f"PyPI API error: {message}", ErrorKind.PYPI_API)


class NpmApiError(ProcessorError):
    """NPM API errors."""
    
    def __init__(self, message: str, status_code: Optional[int] = None):
        self.status_code = status_code
        super().__init__(f"NPM API error: {message}", ErrorKind.NPM_API)


class CacheError(ProcessorError):
    """Cache errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"Cache error: {message}", ErrorKind.CACHE, source)


class DownloadError(ProcessorError):
    """File download errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"Download error: {message}", ErrorKind.DOWNLOAD, source)


class ZipError(ProcessorError):
    """ZIP file processing errors."""
    
    def __init__(self, message: str, source: Optional[Exception] = None):
        super().__init__(f"Zip error: {message}", ErrorKind.ZIP, source)
