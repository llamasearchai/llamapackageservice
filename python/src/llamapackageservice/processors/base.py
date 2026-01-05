"""
Base processor interface for all package processors.
"""

from abc import ABC, abstractmethod
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..config import Config


class PackageProcessor(ABC):
    """
    Abstract base class for package processors.
    
    All processors must implement this interface to handle
    different types of package sources.
    """
    
    @abstractmethod
    async def process(self, url: str, output_dir: Path, config: "Config") -> None:
        """
        Process a package from the given URL and write output to the specified directory.
        
        Args:
            url: The URL or path of the package to process
            output_dir: Directory to save output files
            config: Application configuration
        """
        pass
    
    @abstractmethod
    def name(self) -> str:
        """
        Return the name of the processor.
        
        Returns:
            Human-readable name for this processor type
        """
        pass
    
    @abstractmethod
    def accepts(self, url: str) -> bool:
        """
        Determine if this processor can handle the given URL.
        
        Args:
            url: The URL to check
            
        Returns:
            True if this processor can handle the URL
        """
        pass
    
    @abstractmethod
    async def validate(self, url: str) -> None:
        """
        Validate that the URL can be processed before starting.
        
        Args:
            url: The URL to validate
            
        Raises:
            ValidationError: If the URL is invalid for this processor
        """
        pass
