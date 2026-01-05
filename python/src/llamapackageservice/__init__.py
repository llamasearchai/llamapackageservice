"""
LlamaPackageService - A comprehensive package processing and analysis tool

This library provides functionality to process and analyze code repositories from various sources
including GitHub, PyPI, NPM registries, and local directories. It can generate comprehensive
documentation, perform security audits, and provide AI-powered code analysis.

Features:
    - Process GitHub repositories and organizations
    - Process PyPI packages
    - Process NPM packages
    - Process Rust crates
    - Analytics and metrics collection
    - Caching for improved performance

Usage:
    from llamapackageservice import Config, ProcessorFactory
    import asyncio

    async def main():
        config = Config(output_dir="./output")
        processor = ProcessorFactory.create_processor("https://github.com/rust-lang/rust")
        await processor.process("https://github.com/rust-lang/rust", config.output_dir, config)

    asyncio.run(main())
"""

__version__ = "0.1.0"
__author__ = "Nik Jois <nikjois@llamasearch.ai>"

from .config import Config, ProcessingConfig, RateLimits, OutputConfig, ApiKeys
from .error import (
    ProcessorError,
    IOError,
    HttpError,
    ValidationError,
    NetworkError,
    RateLimitError,
    ConfigError,
    CacheError,
)
from .cache import Cache, FileCache, CacheEntry
from .processors import ProcessorFactory, PackageProcessor
from .processors.github import GitHubProcessor
from .processors.pypi import PyPiProcessor
from .processors.npm import NpmProcessor
from .processors.crates import CratesProcessor
from .processors.go import GoProcessor
from .processors.local import LocalProcessor
from .output_organizer import OutputPaths, list_output_files, organize_output, generate_index
from .agents import OpenAIAgent, AnalysisRequest, AnalysisResult, AnalysisType
from .parallel import ParallelProcessor
from .utils import normalize_url_or_path, normalize_user_input_path

__all__ = [
    # Version info
    "__version__",
    "__author__",
    # Config
    "Config",
    "ProcessingConfig",
    "RateLimits",
    "OutputConfig",
    "ApiKeys",
    # Errors
    "ProcessorError",
    "IOError",
    "HttpError",
    "ValidationError",
    "NetworkError",
    "RateLimitError",
    "ConfigError",
    "CacheError",
    # Cache
    "Cache",
    "FileCache",
    "CacheEntry",
    # Processors
    "ProcessorFactory",
    "PackageProcessor",
    "GitHubProcessor",
    "PyPiProcessor",
    "NpmProcessor",
    "CratesProcessor",
    "GoProcessor",
    "LocalProcessor",
    # Output
    "OutputPaths",
    "list_output_files",
    "organize_output",
    "generate_index",
    # Agents
    "OpenAIAgent",
    "AnalysisRequest",
    "AnalysisResult",
    "AnalysisType",
    # Parallel
    "ParallelProcessor",
    # Utils
    "normalize_url_or_path",
    "normalize_user_input_path",
]


def limit_memory_usage():
    """
    Placeholder for memory limiting functionality.
    In Python, this can be implemented using resource limits on Unix systems.
    """
    import resource
    import sys

    if sys.platform != "win32":
        try:
            # Set soft limit to 4GB, hard limit to system max
            soft, hard = resource.getrlimit(resource.RLIMIT_AS)
            resource.setrlimit(resource.RLIMIT_AS, (4 * 1024 * 1024 * 1024, hard))
        except (ValueError, resource.error):
            pass  # Ignore errors on platforms that don't support this
