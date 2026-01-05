"""
NPM package processor.

Handles processing of JavaScript/TypeScript packages from the NPM registry.
"""

import re
import tarfile
import tempfile
from pathlib import Path
from typing import Optional, Dict, Any
from datetime import datetime
import logging

import httpx
import aiofiles
import aiofiles.os

from ..config import Config
from ..error import ProcessorError, NpmApiError, ValidationError, HttpError
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file

logger = logging.getLogger(__name__)

NPM_REGISTRY_API = "https://registry.npmjs.org"

# File extensions to process
CODE_EXTENSIONS = {'.js', '.ts', '.jsx', '.tsx', '.mjs', '.cjs', '.vue', '.svelte'}

# Directories to skip
SKIP_DIRS = {'node_modules', '.git', 'dist', 'build', 'coverage', '.nyc_output'}


class NpmProcessor(PackageProcessor):
    """
    Processor for handling NPM (Node Package Manager) packages.
    
    This processor can extract information from NPM package URLs or npm install
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
        return "NPM Package"
    
    def accepts(self, url: str) -> bool:
        url_lower = url.lower()
        return (
            "npmjs.com/package/" in url_lower or
            "npm install" in url_lower or
            url.startswith("npm ")
        )
    
    async def validate(self, url: str) -> None:
        """Validate the NPM URL or npm command."""
        package_name = self._extract_package_name(url)
        if not package_name:
            raise ValidationError(f"Could not extract package name from: {url}")
        
        # Validate package exists
        client = await self._ensure_client()
        api_url = f"{NPM_REGISTRY_API}/{package_name}"
        
        response = await client.get(api_url)
        if response.status_code == 404:
            raise ValidationError(f"Package not found on NPM: {package_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to validate package: {response.status_code}")
    
    def _extract_package_name(self, url: str) -> Optional[str]:
        """
        Extract package name from various input formats.
        
        Supports:
        - https://www.npmjs.com/package/package-name
        - https://www.npmjs.com/package/@scope/package-name
        - npm install package-name
        - npm install @scope/package-name
        """
        # From NPM URL (including scoped packages)
        match = re.search(r'npmjs\.com/package/((?:@[^/]+/)?[^/\s]+)', url)
        if match:
            return match.group(1)
        
        # From npm command (including scoped packages)
        match = re.search(r'npm\s+install\s+((?:@[^/\s]+/)?[^\s@<>=!]+)', url)
        if match:
            return match.group(1)
        
        # Direct package name
        if not ' ' in url and ('/' not in url or url.startswith('@')):
            return url.strip()
        
        return None
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process an NPM package.
        
        Args:
            url: NPM URL or npm install command
            output_dir: Directory to save output
            config: Application configuration
        """
        client = await self._ensure_client()
        package_name = self._extract_package_name(url)
        
        if not package_name:
            raise ValidationError(f"Could not extract package name from: {url}")
        
        logger.info(f"Processing NPM package: {package_name}")
        
        # Fetch package info
        package_info = await self._fetch_package_info(client, package_name)
        
        # Create output directory
        npm_dir = output_dir / "npm_packages"
        npm_dir.mkdir(parents=True, exist_ok=True)
        
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
            version = package_info.get('version', 'unknown')
            safe_name = sanitize_filename(package_name.replace('@', '').replace('/', '_'))
            output_filename = f"{timestamp}_{safe_name}_{version}_analysis.txt"
            
            await save_output_file(analysis, npm_dir, output_filename)
            
            logger.info(f"Package analysis saved to {npm_dir / output_filename}")
    
    async def _fetch_package_info(
        self,
        client: httpx.AsyncClient,
        package_name: str,
    ) -> Dict[str, Any]:
        """Fetch package information from NPM registry."""
        # URL encode scoped packages
        encoded_name = package_name.replace('/', '%2F')
        url = f"{NPM_REGISTRY_API}/{encoded_name}"
        
        response = await client.get(url)
        
        if response.status_code == 404:
            raise ValidationError(f"Package not found: {package_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to fetch package info: {response.status_code}")
        
        data = response.json()
        
        # Get the latest version info
        dist_tags = data.get('dist-tags', {})
        latest_version = dist_tags.get('latest', '')
        versions = data.get('versions', {})
        
        if latest_version and latest_version in versions:
            version_info = versions[latest_version]
            version_info['all_versions'] = list(versions.keys())
            return version_info
        
        return data
    
    async def _download_package(
        self,
        client: httpx.AsyncClient,
        package_info: Dict[str, Any],
        temp_dir: Path,
    ) -> Path:
        """Download the package tarball."""
        dist = package_info.get('dist', {})
        tarball_url = dist.get('tarball')
        
        if not tarball_url:
            raise ProcessorError("No tarball URL found in package info")
        
        logger.info(f"Downloading package from {tarball_url}")
        
        response = await client.get(tarball_url, follow_redirects=True)
        
        if not response.is_success:
            raise HttpError(f"Failed to download package: {response.status_code}")
        
        filename = tarball_url.split('/')[-1]
        archive_path = temp_dir / filename
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _extract_package(self, archive_path: Path, extract_dir: Path) -> None:
        """Extract the downloaded package tarball."""
        extract_dir.mkdir(parents=True, exist_ok=True)
        
        import asyncio
        
        def extract():
            with tarfile.open(archive_path, 'r:gz') as tf:
                tf.extractall(extract_dir)
        
        await asyncio.to_thread(extract)
    
    async def _analyze_package(
        self,
        extract_dir: Path,
        package_info: Dict[str, Any],
    ) -> str:
        """Analyze a package and generate documentation."""
        sections = []
        
        # Header
        name = package_info.get('name', 'Unknown')
        version = package_info.get('version', 'Unknown')
        
        sections.append(f"# {name} v{version}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        
        # Package metadata
        sections.append("\n## Package Information\n")
        sections.append(f"- **Name**: {name}")
        sections.append(f"- **Version**: {version}")
        sections.append(f"- **Description**: {package_info.get('description', 'N/A')}")
        sections.append(f"- **Author**: {package_info.get('author', 'Unknown')}")
        sections.append(f"- **License**: {package_info.get('license', 'Unknown')}")
        
        if package_info.get('homepage'):
            sections.append(f"- **Homepage**: {package_info['homepage']}")
        if package_info.get('repository'):
            repo = package_info['repository']
            if isinstance(repo, dict):
                sections.append(f"- **Repository**: {repo.get('url', 'N/A')}")
            else:
                sections.append(f"- **Repository**: {repo}")
        
        # Keywords
        keywords = package_info.get('keywords', [])
        if keywords:
            sections.append(f"- **Keywords**: {', '.join(keywords)}")
        
        # Dependencies
        deps = package_info.get('dependencies', {})
        dev_deps = package_info.get('devDependencies', {})
        
        if deps:
            sections.append("\n## Dependencies\n")
            for dep, version in deps.items():
                sections.append(f"- {dep}: {version}")
        
        if dev_deps:
            sections.append("\n## Dev Dependencies\n")
            for dep, version in dev_deps.items():
                sections.append(f"- {dep}: {version}")
        
        # Find the package root (NPM tarballs extract to 'package' directory)
        package_root = extract_dir / "package"
        if not package_root.exists():
            # Try to find it
            subdirs = list(extract_dir.iterdir())
            if subdirs:
                package_root = subdirs[0]
            else:
                package_root = extract_dir
        
        # README
        readme = await self._find_readme(package_root)
        if readme:
            sections.append(f"\n## README\n\n{readme}")
        
        # File structure
        structure = await self._build_file_tree(package_root)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(package_root)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _find_readme(self, package_root: Path) -> Optional[str]:
        """Find and read the README file."""
        readme_names = ['README.md', 'README.rst', 'README.txt', 'README', 'readme.md', 'Readme.md']
        
        for name in readme_names:
            readme_path = package_root / name
            if readme_path.exists():
                try:
                    async with aiofiles.open(readme_path, 'r', encoding='utf-8') as f:
                        return await f.read()
                except Exception:
                    continue
        
        return None
    
    async def _build_file_tree(self, root: Path, prefix: str = "", max_depth: int = 4) -> str:
        """Build a string representation of the file tree."""
        lines = []
        
        try:
            items = sorted(root.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower()))
        except PermissionError:
            return f"{prefix}[Permission denied]"
        
        for i, item in enumerate(items):
            if item.name in SKIP_DIRS or item.name.startswith('.'):
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
        """Collect JavaScript/TypeScript source code from the package."""
        code_sections = []
        file_count = 0
        
        for path in root.rglob('*'):
            if file_count >= max_files:
                code_sections.append(f"\n... and more files (limit reached)")
                break
            
            if not path.is_file():
                continue
            
            # Skip ignored directories
            if any(skip in path.parts for skip in SKIP_DIRS):
                continue
            
            ext = path.suffix.lower()
            if ext not in CODE_EXTENSIONS:
                continue
            
            try:
                async with aiofiles.open(path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = await f.read()
                
                # Skip very large files
                if len(content) > 100000:
                    continue
                
                rel_path = path.relative_to(root)
                lang = 'typescript' if ext in {'.ts', '.tsx'} else 'javascript'
                
                code_sections.append(f"\n### {rel_path}\n\n```{lang}\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
