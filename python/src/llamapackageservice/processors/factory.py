"""
Factory for creating package processors based on URL type.
"""

from typing import Type

from ..error import ProcessorError
from ..utils.path import is_local_path
from .base import PackageProcessor


class ProcessorFactory:
    """Factory for creating package processors."""
    
    @staticmethod
    def create_processor(url: str) -> PackageProcessor:
        """
        Create the appropriate processor for the given URL.
        
        Args:
            url: The URL or path to process
            
        Returns:
            The appropriate processor instance
            
        Raises:
            ProcessorError: If no processor can handle the URL
        """
        # Import here to avoid circular imports
        from .local import LocalProcessor
        from .github import GitHubProcessor
        from .pypi import PyPiProcessor
        from .npm import NpmProcessor
        from .crates import CratesProcessor
        from .go import GoProcessor
        
        url_lower = url.lower()
        
        # Local file/directory detection (check first)
        if is_local_path(url):
            return LocalProcessor()
        
        # Go package detection
        if "pkg.go.dev" in url_lower or "golang.org/pkg" in url_lower:
            return GoProcessor()
        
        # GitHub repository detection
        if "github.com" in url_lower:
            return GitHubProcessor()
        
        # PyPI package detection
        if "pypi.org/project/" in url_lower or "pip install" in url_lower or url.startswith("pip "):
            return PyPiProcessor()
        
        # NPM package detection
        if "npmjs.com" in url_lower or "npm install" in url_lower or url.startswith("npm "):
            return NpmProcessor()
        
        # Rust crate detection
        if "crates.io" in url_lower or "docs.rs" in url_lower or url.startswith("cargo add "):
            return CratesProcessor()
        
        # Try to guess from domain
        if "gitlab.com" in url_lower or "bitbucket.org" in url_lower:
            return GitHubProcessor()  # Treat as similar to GitHub
        
        raise ProcessorError(
            f"Unsupported URL: {url}. Please use a URL from a supported source "
            "(GitHub, PyPI, NPM, crates.io, pkg.go.dev) or provide a local file/directory path"
        )
    
    @staticmethod
    def detect_url_type(url: str) -> str:
        """
        Detect the type of URL for informational purposes.
        
        Args:
            url: The URL to analyze
            
        Returns:
            Human-readable description of the URL type
        """
        url_lower = url.lower()
        
        if is_local_path(url):
            return "Local File/Directory"
        elif "github.com" in url_lower:
            # Check if it's an organization
            parts = url_lower.replace("https://", "").replace("http://", "").split("/")
            if len(parts) == 2:
                return "GitHub Organization"
            return "GitHub Repository"
        elif "pypi.org/user/" in url_lower:
            return "PyPI User Profile"
        elif "pypi.org" in url_lower or "pip install" in url_lower:
            return "PyPI Package"
        elif "npmjs.com" in url_lower or "npm install" in url_lower:
            return "NPM Package"
        elif "crates.io" in url_lower or "docs.rs" in url_lower:
            return "Rust Crate"
        elif "pkg.go.dev" in url_lower or "golang.org/pkg" in url_lower:
            return "Go Package"
        elif "gitlab.com" in url_lower:
            return "GitLab Repository"
        elif "bitbucket.org" in url_lower:
            return "Bitbucket Repository"
        else:
            return "Unknown"
