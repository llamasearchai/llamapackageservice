"""
Pytest configuration and fixtures.
"""

import pytest
import sys
from pathlib import Path

# Add src directory to path for imports
src_path = Path(__file__).parent.parent / "src"
sys.path.insert(0, str(src_path))


def pytest_configure(config):
    """Configure pytest markers."""
    config.addinivalue_line(
        "markers", "integration: mark test as integration test (requires network)"
    )


@pytest.fixture
def sample_github_urls():
    """Sample GitHub URLs for testing."""
    return [
        "https://github.com/python/cpython",
        "https://github.com/microsoft/vscode",
        "https://github.com/rust-lang/rust",
    ]


@pytest.fixture
def sample_pypi_packages():
    """Sample PyPI packages for testing."""
    return [
        "requests",
        "flask",
        "numpy",
    ]


@pytest.fixture
def sample_npm_packages():
    """Sample NPM packages for testing."""
    return [
        "lodash",
        "express",
        "react",
    ]
