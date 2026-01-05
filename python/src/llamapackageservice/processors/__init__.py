"""
Package processors for different sources (GitHub, PyPI, NPM, etc.).
"""

from .base import PackageProcessor
from .factory import ProcessorFactory
from .github import GitHubProcessor
from .pypi import PyPiProcessor
from .npm import NpmProcessor
from .crates import CratesProcessor
from .go import GoProcessor
from .local import LocalProcessor

__all__ = [
    "PackageProcessor",
    "ProcessorFactory",
    "GitHubProcessor",
    "PyPiProcessor",
    "NpmProcessor",
    "CratesProcessor",
    "GoProcessor",
    "LocalProcessor",
]
