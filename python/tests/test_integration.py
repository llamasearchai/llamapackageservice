"""
Integration tests for LlamaPackageService.
"""

import pytest
from pathlib import Path
import tempfile
import asyncio

from llamapackageservice import process_url, Config
from llamapackageservice.processors import ProcessorFactory, ProcessorType


class TestProcessorIntegration:
    """Integration tests for processors."""
    
    @pytest.fixture
    def temp_output_dir(self):
        """Create a temporary output directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)
    
    @pytest.fixture
    def config(self, temp_output_dir):
        """Create a test configuration."""
        return Config(output_dir=temp_output_dir)
    
    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_process_github_repo(self, temp_output_dir, config):
        """Test processing a GitHub repository."""
        url = "https://github.com/python/cpython"
        
        # This test requires network access
        processor = ProcessorFactory.create_processor(url)
        await processor.validate(url)
        
        # Note: Full processing would take too long for unit tests
        # This just validates the URL and processor creation
        assert processor.name() == "github"
    
    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_process_pypi_package(self, temp_output_dir, config):
        """Test processing a PyPI package."""
        url = "https://pypi.org/project/requests"
        
        processor = ProcessorFactory.create_processor(url)
        await processor.validate(url)
        
        assert processor.name() == "pypi"
    
    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_process_npm_package(self, temp_output_dir, config):
        """Test processing an NPM package."""
        url = "https://www.npmjs.com/package/lodash"
        
        processor = ProcessorFactory.create_processor(url)
        await processor.validate(url)
        
        assert processor.name() == "npm"
    
    @pytest.mark.asyncio
    async def test_process_local_directory(self, temp_output_dir, config):
        """Test processing a local directory."""
        # Create a test directory with some files
        test_dir = temp_output_dir / "test_project"
        test_dir.mkdir()
        
        (test_dir / "main.py").write_text("print('hello')")
        (test_dir / "README.md").write_text("# Test Project")
        
        processor = ProcessorFactory.create_processor(str(test_dir))
        await processor.validate(str(test_dir))
        
        assert processor.name() == "local"


class TestURLDetection:
    """Tests for URL type detection."""
    
    def test_various_github_formats(self):
        """Test various GitHub URL formats."""
        test_cases = [
            ("https://github.com/user/repo", ProcessorType.GITHUB_REPO),
            ("https://github.com/user/repo.git", ProcessorType.GITHUB_REPO),
            ("http://github.com/user/repo", ProcessorType.GITHUB_REPO),
            ("github.com/user/repo", ProcessorType.GITHUB_REPO),
            ("https://github.com/organization", ProcessorType.GITHUB_ORG),
        ]
        
        for url, expected in test_cases:
            result = ProcessorFactory.detect_url_type(url)
            assert result == expected, f"Failed for {url}: expected {expected}, got {result}"
    
    def test_various_pypi_formats(self):
        """Test various PyPI URL formats."""
        test_cases = [
            ("https://pypi.org/project/requests", ProcessorType.PYPI),
            ("https://pypi.python.org/pypi/requests", ProcessorType.PYPI),
            ("pip install requests", ProcessorType.PYPI),
            ("pip3 install numpy", ProcessorType.PYPI),
        ]
        
        for url, expected in test_cases:
            result = ProcessorFactory.detect_url_type(url)
            assert result == expected, f"Failed for {url}: expected {expected}, got {result}"
    
    def test_various_npm_formats(self):
        """Test various NPM URL formats."""
        test_cases = [
            ("https://www.npmjs.com/package/lodash", ProcessorType.NPM),
            ("https://npmjs.org/package/express", ProcessorType.NPM),
            ("npm install react", ProcessorType.NPM),
            ("npm i vue", ProcessorType.NPM),
        ]
        
        for url, expected in test_cases:
            result = ProcessorFactory.detect_url_type(url)
            assert result == expected, f"Failed for {url}: expected {expected}, got {result}"
    
    def test_local_paths(self):
        """Test local path detection."""
        test_cases = [
            ("/home/user/project", ProcessorType.LOCAL),
            ("./my-project", ProcessorType.LOCAL),
            ("../other-project", ProcessorType.LOCAL),
            ("~/Documents/code", ProcessorType.LOCAL),
            ("C:\\Users\\project", ProcessorType.LOCAL),
        ]
        
        for path, expected in test_cases:
            result = ProcessorFactory.detect_url_type(path)
            assert result == expected, f"Failed for {path}: expected {expected}, got {result}"


class TestConfigIntegration:
    """Integration tests for configuration."""
    
    def test_config_with_output_dir(self):
        """Test configuration with custom output directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config = Config(output_dir=Path(tmpdir))
            assert config.output_dir == Path(tmpdir)
    
    def test_config_load_save_roundtrip(self):
        """Test configuration load/save roundtrip."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            
            original = Config(output_dir=Path(tmpdir) / "output")
            original.processing.max_concurrent = 8
            original.save(config_path)
            
            loaded = Config.load(config_path)
            assert loaded.processing.max_concurrent == 8


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
