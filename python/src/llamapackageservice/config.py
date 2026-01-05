"""
Configuration module for the application.

This module holds all configuration settings including API tokens,
output directories, processing limits, and other settings.
"""

from pathlib import Path
from typing import Optional, List
from datetime import timedelta
import os
import re

from pydantic import BaseModel, Field
from pydantic_settings import BaseSettings

from .error import ConfigError, ProcessorError


class ApiKeys(BaseModel):
    """API keys for various services."""
    
    github_token: Optional[str] = None
    openai_api_key: Optional[str] = None
    deepseek_api_key: Optional[str] = None
    
    @classmethod
    def from_env(cls) -> "ApiKeys":
        """Create ApiKeys from environment variables."""
        return cls(
            github_token=os.getenv("GITHUB_TOKEN"),
            openai_api_key=os.getenv("OPENAI_API_KEY"),
            deepseek_api_key=os.getenv("DEEPSEEK_API_KEY"),
        )


class ProcessingConfig(BaseModel):
    """Configuration for parallel processing operations."""
    
    max_concurrent_downloads: int = Field(default=5, ge=1, le=50)
    max_concurrent_extractions: int = Field(default=3, ge=1, le=20)
    max_concurrent_analyses: int = Field(default=3, ge=1, le=20)


class RateLimits(BaseModel):
    """Rate limit settings for various APIs."""
    
    github_api: int = Field(default=5000, description="GitHub API rate limit (requests per hour)")
    pypi_api: int = Field(default=100, description="PyPI API rate limit (requests per minute)")
    npm_api: int = Field(default=100, description="NPM API rate limit (requests per minute)")


class OutputConfig(BaseModel):
    """Configuration for output files and directories."""
    
    base_dir: Path = Field(default_factory=lambda: Path("./output"))
    temp_dir: Path = Field(default_factory=lambda: Path("./output/_temp"))
    cache_duration: timedelta = Field(default=timedelta(hours=24))


class Config(BaseModel):
    """
    Main configuration struct for the application.
    
    This structure holds all configuration settings including API tokens,
    output directories, processing limits, and other settings.
    """
    
    github_token: Optional[str] = Field(default=None, description="GitHub API token")
    output_dir: Path = Field(default_factory=lambda: Path("./output"))
    processing: ProcessingConfig = Field(default_factory=ProcessingConfig)
    rate_limits: RateLimits = Field(default_factory=RateLimits)
    output_config: OutputConfig = Field(default_factory=OutputConfig)
    api_keys: ApiKeys = Field(default_factory=ApiKeys.from_env)
    excluded_files: List[str] = Field(
        default_factory=lambda: [
            r"\.git/",
            r"node_modules/",
            r"\.env",
        ]
    )
    
    class Config:
        arbitrary_types_allowed = True
    
    def __init__(self, output_dir: Optional[Path] = None, **data):
        """Create a new configuration with the specified output directory."""
        if output_dir is not None:
            data["output_dir"] = Path(output_dir)
        super().__init__(**data)
        
        # Update github_token from environment if not set
        if self.github_token is None:
            self.github_token = os.getenv("GITHUB_TOKEN")
        
        # Update output_config base_dir
        self.output_config.base_dir = self.output_dir
    
    @classmethod
    def load(cls) -> "Config":
        """
        Load configuration from the default config file location.
        
        If the config file doesn't exist, returns the default configuration.
        The config file is expected to be in TOML format.
        """
        config_dir = Path.home() / ".config" / "llama-package-service"
        config_path = config_dir / "config.toml"
        
        if not config_path.exists():
            return cls()
        
        try:
            import toml
            content = toml.load(config_path)
            return cls(**content)
        except Exception as e:
            raise ConfigError(f"Failed to load config file: {e}")
    
    async def validate(self) -> None:
        """
        Validate the configuration by ensuring necessary directories exist
        and API tokens are valid.
        """
        await self.ensure_directories_exist()
        self.ensure_tokens()
    
    async def ensure_directories_exist(self) -> None:
        """
        Ensure all output directories required by the application exist.
        Creates any missing directories to prepare for file output operations.
        """
        import aiofiles.os
        
        if not await aiofiles.os.path.exists(self.output_dir):
            os.makedirs(self.output_dir, exist_ok=True)
    
    def ensure_tokens(self) -> None:
        """
        Check that required API tokens are configured.
        Note: This is a soft check - tokens may not be needed for all operations.
        """
        # Just a validation pass - we don't require tokens for all operations
        pass
    
    def is_excluded_file(self, path: Path) -> bool:
        """
        Check if a file should be excluded from processing based on configured patterns.
        
        Args:
            path: The file path to check against the exclusion patterns.
            
        Returns:
            True if the file should be excluded, False otherwise.
        """
        path_str = str(path)
        for pattern in self.excluded_files:
            try:
                if re.search(pattern, path_str):
                    return True
            except re.error:
                continue
        return False
    
    def get_output_path(self, package_type: str, name: str) -> Path:
        """
        Get the appropriate output path for a given package type.
        
        Args:
            package_type: The type of package (github, pypi, npm, etc.)
            name: The name of the package
            
        Returns:
            Path to the output file
        """
        from datetime import datetime
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        
        type_dirs = {
            "github_repo": "github_repos",
            "github_org": "github_orgs",
            "pypi": "pypi_packages",
            "pypi_profile": "pypi_profiles",
            "npm": "npm_packages",
            "crate": "rust_crates",
            "go": "go_packages",
            "local": "local_repositories",
        }
        
        subdir = type_dirs.get(package_type, "other")
        output_subdir = self.output_dir / subdir
        output_subdir.mkdir(parents=True, exist_ok=True)
        
        safe_name = re.sub(r'[^\w\-_.]', '_', name)
        filename = f"{timestamp}_{safe_name}_analysis.txt"
        
        return output_subdir / filename
