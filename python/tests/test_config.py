"""
Tests for configuration module.
"""

import pytest
from pathlib import Path
import tempfile
import json

from llamapackageservice.config import (
    Config,
    ProcessingConfig,
    RateLimits,
    OutputConfig,
    ApiKeys,
)


class TestProcessingConfig:
    """Tests for ProcessingConfig."""
    
    def test_default_values(self):
        """Test default configuration values."""
        config = ProcessingConfig()
        
        assert config.max_concurrent == 4
        assert config.retry_attempts == 3
        assert config.timeout == 30.0
        assert config.verbose is False
    
    def test_custom_values(self):
        """Test custom configuration values."""
        config = ProcessingConfig(
            max_concurrent=8,
            retry_attempts=5,
            timeout=60.0,
            verbose=True,
        )
        
        assert config.max_concurrent == 8
        assert config.retry_attempts == 5
        assert config.timeout == 60.0
        assert config.verbose is True


class TestRateLimits:
    """Tests for RateLimits."""
    
    def test_default_values(self):
        """Test default rate limit values."""
        limits = RateLimits()
        
        assert limits.github_rpm == 60
        assert limits.pypi_rpm == 100
        assert limits.npm_rpm == 100
        assert limits.crates_rpm == 100
        assert limits.go_rpm == 100
    
    def test_custom_values(self):
        """Test custom rate limit values."""
        limits = RateLimits(
            github_rpm=30,
            pypi_rpm=50,
        )
        
        assert limits.github_rpm == 30
        assert limits.pypi_rpm == 50


class TestOutputConfig:
    """Tests for OutputConfig."""
    
    def test_default_values(self):
        """Test default output configuration."""
        config = OutputConfig()
        
        assert config.generate_index is True
        assert config.organize_by_type is True
        assert config.include_metadata is True
    
    def test_custom_values(self):
        """Test custom output configuration."""
        config = OutputConfig(
            generate_index=False,
            include_metadata=False,
        )
        
        assert config.generate_index is False
        assert config.include_metadata is False


class TestApiKeys:
    """Tests for ApiKeys."""
    
    def test_default_values(self):
        """Test default API keys (all None)."""
        keys = ApiKeys()
        
        assert keys.github_token is None
        assert keys.openai_api_key is None
    
    def test_custom_values(self):
        """Test custom API keys."""
        keys = ApiKeys(
            github_token="ghp_test",
            openai_api_key="sk-test",
        )
        
        assert keys.github_token == "ghp_test"
        assert keys.openai_api_key == "sk-test"


class TestConfig:
    """Tests for Config."""
    
    def test_default_config(self):
        """Test default configuration."""
        config = Config()
        
        assert config.output_dir == Path("output")
        assert isinstance(config.processing, ProcessingConfig)
        assert isinstance(config.rate_limits, RateLimits)
        assert isinstance(config.output, OutputConfig)
        assert isinstance(config.api_keys, ApiKeys)
    
    def test_custom_output_dir(self):
        """Test custom output directory."""
        config = Config(output_dir=Path("/tmp/custom"))
        
        assert config.output_dir == Path("/tmp/custom")
    
    def test_load_from_json(self):
        """Test loading configuration from JSON file."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            config_data = {
                "output_dir": "/tmp/test",
                "processing": {
                    "max_concurrent": 8,
                    "verbose": True,
                },
                "rate_limits": {
                    "github_rpm": 30,
                },
            }
            json.dump(config_data, f)
            f.flush()
            
            config = Config.load(Path(f.name))
            
            assert config.output_dir == Path("/tmp/test")
            assert config.processing.max_concurrent == 8
            assert config.processing.verbose is True
            assert config.rate_limits.github_rpm == 30
    
    def test_load_missing_file(self):
        """Test loading from missing file returns default config."""
        config = Config.load(Path("/nonexistent/path/config.json"))
        
        assert config.output_dir == Path("output")
    
    def test_save_config(self):
        """Test saving configuration to file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            
            config = Config(
                output_dir=Path("/tmp/test"),
                processing=ProcessingConfig(max_concurrent=8),
            )
            config.save(config_path)
            
            # Reload and verify
            loaded = Config.load(config_path)
            assert loaded.output_dir == Path("/tmp/test")
            assert loaded.processing.max_concurrent == 8


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
