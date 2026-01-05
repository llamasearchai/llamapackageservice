"""
Tests for processors module.
"""

import pytest
from pathlib import Path

from llamapackageservice.processors import ProcessorFactory, ProcessorType
from llamapackageservice.processors.base import PackageProcessor
from llamapackageservice.processors.github import GitHubProcessor
from llamapackageservice.processors.pypi import PyPiProcessor
from llamapackageservice.processors.npm import NpmProcessor
from llamapackageservice.processors.crates import CratesProcessor
from llamapackageservice.processors.go import GoProcessor
from llamapackageservice.processors.local import LocalProcessor


class TestProcessorFactory:
    """Tests for ProcessorFactory."""
    
    def test_detect_github_repo(self):
        """Test detection of GitHub repository URLs."""
        urls = [
            "https://github.com/user/repo",
            "https://github.com/user/repo.git",
            "http://github.com/organization/project",
            "github.com/user/repo",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.GITHUB_REPO, f"Failed for {url}"
    
    def test_detect_github_org(self):
        """Test detection of GitHub organization URLs."""
        urls = [
            "https://github.com/organization",
            "http://github.com/my-org",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.GITHUB_ORG, f"Failed for {url}"
    
    def test_detect_pypi_url(self):
        """Test detection of PyPI URLs."""
        urls = [
            "https://pypi.org/project/requests",
            "https://pypi.python.org/pypi/numpy",
            "pip install flask",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.PYPI, f"Failed for {url}"
    
    def test_detect_npm_url(self):
        """Test detection of NPM URLs."""
        urls = [
            "https://www.npmjs.com/package/lodash",
            "https://npmjs.org/package/express",
            "npm install react",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.NPM, f"Failed for {url}"
    
    def test_detect_crates_url(self):
        """Test detection of crates.io URLs."""
        urls = [
            "https://crates.io/crates/serde",
            "https://crates.io/crates/tokio",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.CRATES, f"Failed for {url}"
    
    def test_detect_go_url(self):
        """Test detection of Go package URLs."""
        urls = [
            "https://pkg.go.dev/github.com/gin-gonic/gin",
            "https://pkg.go.dev/golang.org/x/net",
        ]
        
        for url in urls:
            assert ProcessorFactory.detect_url_type(url) == ProcessorType.GO, f"Failed for {url}"
    
    def test_detect_local_path(self):
        """Test detection of local paths."""
        paths = [
            "/home/user/project",
            "./my-project",
            "../other-project",
            "~/Documents/code",
        ]
        
        for path in paths:
            assert ProcessorFactory.detect_url_type(path) == ProcessorType.LOCAL, f"Failed for {path}"
    
    def test_create_github_processor(self):
        """Test creating GitHub processor."""
        processor = ProcessorFactory.create_processor("https://github.com/user/repo")
        assert isinstance(processor, GitHubProcessor)
    
    def test_create_pypi_processor(self):
        """Test creating PyPI processor."""
        processor = ProcessorFactory.create_processor("https://pypi.org/project/requests")
        assert isinstance(processor, PyPiProcessor)
    
    def test_create_npm_processor(self):
        """Test creating NPM processor."""
        processor = ProcessorFactory.create_processor("https://www.npmjs.com/package/lodash")
        assert isinstance(processor, NpmProcessor)
    
    def test_create_crates_processor(self):
        """Test creating Crates processor."""
        processor = ProcessorFactory.create_processor("https://crates.io/crates/serde")
        assert isinstance(processor, CratesProcessor)
    
    def test_create_go_processor(self):
        """Test creating Go processor."""
        processor = ProcessorFactory.create_processor("https://pkg.go.dev/github.com/gin-gonic/gin")
        assert isinstance(processor, GoProcessor)
    
    def test_create_local_processor(self):
        """Test creating Local processor."""
        processor = ProcessorFactory.create_processor("/home/user/project")
        assert isinstance(processor, LocalProcessor)


class TestProcessorInterface:
    """Tests for processor interface."""
    
    def test_processor_has_required_methods(self):
        """Test that processors have required interface methods."""
        processor = GitHubProcessor()
        
        assert hasattr(processor, 'validate')
        assert hasattr(processor, 'process')
        assert hasattr(processor, 'name')
        assert callable(processor.validate)
        assert callable(processor.process)
    
    def test_processor_name(self):
        """Test processor names."""
        assert GitHubProcessor().name() == "github"
        assert PyPiProcessor().name() == "pypi"
        assert NpmProcessor().name() == "npm"
        assert CratesProcessor().name() == "crates"
        assert GoProcessor().name() == "go"
        assert LocalProcessor().name() == "local"


class TestGitHubProcessor:
    """Tests for GitHubProcessor."""
    
    def test_extract_repo_info(self):
        """Test extracting repository info from URL."""
        processor = GitHubProcessor()
        
        owner, repo = processor._extract_repo_info("https://github.com/microsoft/vscode")
        assert owner == "microsoft"
        assert repo == "vscode"
    
    def test_extract_repo_info_with_git_suffix(self):
        """Test extracting repo info from URL with .git suffix."""
        processor = GitHubProcessor()
        
        owner, repo = processor._extract_repo_info("https://github.com/user/repo.git")
        assert owner == "user"
        assert repo == "repo"


class TestPyPiProcessor:
    """Tests for PyPiProcessor."""
    
    def test_extract_package_name_from_url(self):
        """Test extracting package name from PyPI URL."""
        processor = PyPiProcessor()
        
        name = processor._extract_package_name("https://pypi.org/project/requests")
        assert name == "requests"
    
    def test_extract_package_name_from_pip_command(self):
        """Test extracting package name from pip command."""
        processor = PyPiProcessor()
        
        name = processor._extract_package_name("pip install flask")
        assert name == "flask"


class TestNpmProcessor:
    """Tests for NpmProcessor."""
    
    def test_extract_package_name_from_url(self):
        """Test extracting package name from NPM URL."""
        processor = NpmProcessor()
        
        name = processor._extract_package_name("https://www.npmjs.com/package/lodash")
        assert name == "lodash"
    
    def test_extract_package_name_from_npm_command(self):
        """Test extracting package name from npm command."""
        processor = NpmProcessor()
        
        name = processor._extract_package_name("npm install express")
        assert name == "express"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
