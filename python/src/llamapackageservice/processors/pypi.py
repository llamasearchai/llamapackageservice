"""
PyPI package processor.

Handles processing of Python packages from the Python Package Index.
"""

import re
import tarfile
import zipfile
import tempfile
from pathlib import Path
from typing import Optional, Dict, Any
from datetime import datetime
import logging

import httpx
import aiofiles
import aiofiles.os

from ..config import Config
from ..error import ProcessorError, PyPiApiError, ValidationError, HttpError
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file, detect_language_from_extension

logger = logging.getLogger(__name__)

PYPI_API_BASE = "https://pypi.org/pypi"

# File extensions to process
CODE_EXTENSIONS = {'.py', '.pyi', '.pyx', '.pxd', '.pxi'}

# Directories to skip
SKIP_DIRS = {'__pycache__', '.git', 'venv', '.venv', 'dist', 'build', '.eggs', '*.egg-info'}


class PyPiProcessor(PackageProcessor):
    """
    Processor for handling PyPI (Python Package Index) packages.
    
    This processor can extract information from PyPI package URLs or pip install
    commands, download the packages, and generate comprehensive documentation.
    """
    
    def __init__(self):
        self.client: Optional[httpx.AsyncClient] = None
    
    async def _ensure_client(self) -> httpx.AsyncClient:
        """Ensure the HTTP client is initialized."""
        if self.client is None:
            self.client = httpx.AsyncClient(timeout=120.0)
        return self.client
    
    def name(self) -> str:
        return "PyPI Package"
    
    def accepts(self, url: str) -> bool:
        url_lower = url.lower()
        return (
            "pypi.org/project/" in url_lower or
            "pip install" in url_lower or
            url.startswith("pip ")
        )
    
    async def validate(self, url: str) -> None:
        """Validate the PyPI URL or pip command."""
        package_name = self._extract_package_name(url)
        if not package_name:
            raise ValidationError(f"Could not extract package name from: {url}")
        
        # Validate package exists
        client = await self._ensure_client()
        api_url = f"{PYPI_API_BASE}/{package_name}/json"
        
        response = await client.get(api_url)
        if response.status_code == 404:
            raise ValidationError(f"Package not found on PyPI: {package_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to validate package: {response.status_code}")
    
    def _extract_package_name(self, url: str) -> Optional[str]:
        """
        Extract package name from various input formats.
        
        Supports:
        - https://pypi.org/project/package-name/
        - pip install package-name
        - pip install package-name==1.0.0
        - Just the package name
        """
        # From PyPI URL
        match = re.search(r'pypi\.org/project/([^/]+)', url)
        if match:
            return match.group(1).strip('/')
        
        # From pip command
        match = re.search(r'pip\s+install\s+([^\s=<>!]+)', url)
        if match:
            return match.group(1)
        
        # Direct package name (no spaces, no slashes)
        if not ' ' in url and not '/' in url:
            return url.strip()
        
        return None
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process a PyPI package.
        
        Args:
            url: PyPI URL or pip install command
            output_dir: Directory to save output
            config: Application configuration
        """
        client = await self._ensure_client()
        package_name = self._extract_package_name(url)
        
        if not package_name:
            raise ValidationError(f"Could not extract package name from: {url}")
        
        logger.info(f"Processing PyPI package: {package_name}")
        
        # Fetch package info
        package_info = await self._fetch_package_info(client, package_name)
        
        # Create output directory
        pypi_dir = output_dir / "pypi_packages"
        pypi_dir.mkdir(parents=True, exist_ok=True)
        
        # Download and extract package
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            
            # Download package
            archive_path = await self._download_package(client, package_info, temp_path)
            
            # Extract package
            extract_dir = temp_path / "extracted"
            await self._extract_package(archive_path, extract_dir)
            
            # Analyze the package
            analysis = await self._analyze_package(extract_dir, package_info)
            
            # Generate output
            timestamp = get_timestamp()
            version = package_info.get('info', {}).get('version', 'unknown')
            output_filename = f"{timestamp}_{sanitize_filename(package_name)}_{version}_analysis.txt"
            
            await save_output_file(analysis, pypi_dir, output_filename)
            
            logger.info(f"Package analysis saved to {pypi_dir / output_filename}")
    
    async def _fetch_package_info(
        self,
        client: httpx.AsyncClient,
        package_name: str,
    ) -> Dict[str, Any]:
        """Fetch package information from PyPI API."""
        url = f"{PYPI_API_BASE}/{package_name}/json"
        
        response = await client.get(url)
        
        if response.status_code == 404:
            raise ValidationError(f"Package not found: {package_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to fetch package info: {response.status_code}")
        
        return response.json()
    
    async def _download_package(
        self,
        client: httpx.AsyncClient,
        package_info: Dict[str, Any],
        temp_dir: Path,
    ) -> Path:
        """Download the package source distribution."""
        info = package_info.get('info', {})
        version = info.get('version', '')
        releases = package_info.get('releases', {})
        
        if not version or version not in releases:
            raise ProcessorError(f"Unable to find release for version: {version}")
        
        release_files = releases[version]
        
        # Prefer source distribution (sdist) over wheel
        sdist = None
        wheel = None
        
        for file_info in release_files:
            package_type = file_info.get('packagetype', '')
            if package_type == 'sdist':
                sdist = file_info
            elif 'wheel' in package_type and wheel is None:
                wheel = file_info
        
        download_info = sdist or wheel
        if not download_info:
            raise ProcessorError("No suitable distribution found")
        
        download_url = download_info.get('url')
        filename = download_info.get('filename')
        
        if not download_url or not filename:
            raise ProcessorError("Missing download URL or filename")
        
        logger.info(f"Downloading package from {download_url}")
        
        response = await client.get(download_url, follow_redirects=True)
        
        if not response.is_success:
            raise HttpError(f"Failed to download package: {response.status_code}")
        
        archive_path = temp_dir / filename
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _extract_package(self, archive_path: Path, extract_dir: Path) -> None:
        """Extract the downloaded package archive."""
        extract_dir.mkdir(parents=True, exist_ok=True)
        
        import asyncio
        
        filename = archive_path.name.lower()
        
        def extract():
            if filename.endswith('.tar.gz') or filename.endswith('.tgz'):
                with tarfile.open(archive_path, 'r:gz') as tf:
                    tf.extractall(extract_dir)
            elif filename.endswith('.zip') or filename.endswith('.whl'):
                with zipfile.ZipFile(archive_path, 'r') as zf:
                    zf.extractall(extract_dir)
            else:
                raise ProcessorError(f"Unsupported archive format: {filename}")
        
        await asyncio.to_thread(extract)
    
    async def _analyze_package(
        self,
        extract_dir: Path,
        package_info: Dict[str, Any],
    ) -> str:
        """Analyze a package and generate documentation."""
        info = package_info.get('info', {})
        sections = []
        
        # Header
        name = info.get('name', 'Unknown')
        version = info.get('version', 'Unknown')
        
        sections.append(f"# {name} v{version}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        
        # Package metadata
        sections.append("\n## Package Information\n")
        sections.append(f"- **Name**: {name}")
        sections.append(f"- **Version**: {version}")
        sections.append(f"- **Author**: {info.get('author', 'Unknown')}")
        sections.append(f"- **License**: {info.get('license', 'Unknown')}")
        sections.append(f"- **Python Requires**: {info.get('requires_python', 'Not specified')}")
        
        if info.get('home_page'):
            sections.append(f"- **Homepage**: {info['home_page']}")
        if info.get('project_urls'):
            sections.append(f"- **Project URLs**: {info['project_urls']}")
        
        # Description
        summary = info.get('summary', '')
        description = info.get('description', '')
        
        if summary:
            sections.append(f"\n## Summary\n\n{summary}")
        
        if description:
            sections.append(f"\n## Description\n\n{description}")
        
        # Dependencies
        requires = info.get('requires_dist', [])
        if requires:
            sections.append("\n## Dependencies\n")
            for dep in requires:
                sections.append(f"- {dep}")
        
        # Find the package root
        package_root = extract_dir
        subdirs = list(extract_dir.iterdir())
        if len(subdirs) == 1 and subdirs[0].is_dir():
            package_root = subdirs[0]
        
        # File structure
        structure = await self._build_file_tree(package_root)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(package_root)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _build_file_tree(self, root: Path, prefix: str = "", max_depth: int = 4) -> str:
        """Build a string representation of the file tree."""
        lines = []
        
        try:
            items = sorted(root.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower()))
        except PermissionError:
            return f"{prefix}[Permission denied]"
        
        for i, item in enumerate(items):
            if any(item.name.endswith(skip.replace('*', '')) or item.name == skip 
                   for skip in SKIP_DIRS):
                continue
            
            is_last = i == len(items) - 1
            connector = "└── " if is_last else "├── "
            lines.append(f"{prefix}{connector}{item.name}")
            
            if item.is_dir() and max_depth > 0:
                extension = "    " if is_last else "│   "
                subtree = await self._build_file_tree(item, prefix + extension, max_depth - 1)
                if subtree:
                    lines.append(subtree)
        
        return "\n".join(lines)
    
    async def _collect_source_code(self, root: Path, max_files: int = 100) -> str:
        """Collect Python source code from the package."""
        code_sections = []
        file_count = 0
        
        for path in root.rglob('*.py'):
            if file_count >= max_files:
                code_sections.append(f"\n... and more files (limit reached)")
                break
            
            # Skip ignored directories
            if any(skip.replace('*', '') in str(path) for skip in SKIP_DIRS):
                continue
            
            try:
                async with aiofiles.open(path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = await f.read()
                
                # Skip very large files
                if len(content) > 100000:
                    continue
                
                rel_path = path.relative_to(root)
                code_sections.append(f"\n### {rel_path}\n\n```python\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
