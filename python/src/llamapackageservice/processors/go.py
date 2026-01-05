"""
Go package processor.

Handles processing of Go packages from pkg.go.dev.
"""

import re
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
from ..error import ProcessorError, ValidationError, HttpError
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file

logger = logging.getLogger(__name__)

GO_PROXY = "https://proxy.golang.org"

# File extensions to process
CODE_EXTENSIONS = {'.go', '.mod', '.sum'}

# Directories to skip
SKIP_DIRS = {'vendor', '.git', 'testdata'}


class GoProcessor(PackageProcessor):
    """
    Processor for handling Go packages from pkg.go.dev.
    
    This processor can extract information from pkg.go.dev URLs,
    download the packages, and generate comprehensive documentation.
    """
    
    def __init__(self):
        self.client: Optional[httpx.AsyncClient] = None
    
    async def _ensure_client(self) -> httpx.AsyncClient:
        """Ensure the HTTP client is initialized."""
        if self.client is None:
            self.client = httpx.AsyncClient(timeout=120.0)
        return self.client
    
    def name(self) -> str:
        return "Go Package"
    
    def accepts(self, url: str) -> bool:
        url_lower = url.lower()
        return "pkg.go.dev" in url_lower or "golang.org/pkg" in url_lower
    
    async def validate(self, url: str) -> None:
        """Validate the Go package URL."""
        module_path = self._extract_module_path(url)
        if not module_path:
            raise ValidationError(f"Could not extract module path from: {url}")
    
    def _extract_module_path(self, url: str) -> Optional[str]:
        """
        Extract Go module path from various URL formats.
        
        Supports:
        - https://pkg.go.dev/github.com/user/repo
        - https://pkg.go.dev/golang.org/x/tools
        - github.com/user/repo
        """
        # From pkg.go.dev URL
        match = re.search(r'pkg\.go\.dev/([^\s?#]+)', url)
        if match:
            return match.group(1)
        
        # From golang.org/pkg URL
        match = re.search(r'golang\.org/pkg/([^\s?#]+)', url)
        if match:
            return match.group(1)
        
        # Direct module path (contains at least one /)
        if '/' in url and not url.startswith('http'):
            return url.strip()
        
        return None
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process a Go package.
        
        Args:
            url: pkg.go.dev URL or module path
            output_dir: Directory to save output
            config: Application configuration
        """
        client = await self._ensure_client()
        module_path = self._extract_module_path(url)
        
        if not module_path:
            raise ValidationError(f"Could not extract module path from: {url}")
        
        logger.info(f"Processing Go package: {module_path}")
        
        # Create output directory
        go_dir = output_dir / "go_packages"
        go_dir.mkdir(parents=True, exist_ok=True)
        
        # Get module version info
        version_info = await self._get_latest_version(client, module_path)
        version = version_info.get('Version', 'latest')
        
        # Download and extract package
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            
            # Download package
            archive_path = await self._download_module(client, module_path, version, temp_path)
            
            # Extract package
            extract_dir = temp_path / "extracted"
            await self._extract_module(archive_path, extract_dir)
            
            # Analyze the package
            analysis = await self._analyze_module(extract_dir, module_path, version_info)
            
            # Generate output
            timestamp = get_timestamp()
            safe_name = sanitize_filename(module_path.replace('/', '_'))
            output_filename = f"{timestamp}_{safe_name}_{version}_analysis.txt"
            
            await save_output_file(analysis, go_dir, output_filename)
            
            logger.info(f"Package analysis saved to {go_dir / output_filename}")
    
    async def _get_latest_version(
        self,
        client: httpx.AsyncClient,
        module_path: str,
    ) -> Dict[str, Any]:
        """Get the latest version info for a Go module."""
        # Try to get the latest version from the Go proxy
        url = f"{GO_PROXY}/{module_path}/@latest"
        
        try:
            response = await client.get(url)
            if response.is_success:
                return response.json()
        except Exception as e:
            logger.debug(f"Could not fetch version info: {e}")
        
        # Return a default
        return {'Version': 'latest'}
    
    async def _download_module(
        self,
        client: httpx.AsyncClient,
        module_path: str,
        version: str,
        temp_dir: Path,
    ) -> Path:
        """Download the Go module archive."""
        # The Go proxy serves modules as zip files
        download_url = f"{GO_PROXY}/{module_path}/@v/{version}.zip"
        
        logger.info(f"Downloading module from {download_url}")
        
        response = await client.get(download_url, follow_redirects=True)
        
        if not response.is_success:
            # Try to download from GitHub if it's a GitHub module
            if module_path.startswith('github.com/'):
                return await self._download_from_github(client, module_path, temp_dir)
            raise HttpError(f"Failed to download module: {response.status_code}")
        
        archive_path = temp_dir / "module.zip"
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _download_from_github(
        self,
        client: httpx.AsyncClient,
        module_path: str,
        temp_dir: Path,
    ) -> Path:
        """Download a Go module directly from GitHub."""
        # Extract owner and repo from module path
        parts = module_path.replace('github.com/', '').split('/')
        if len(parts) < 2:
            raise ValidationError(f"Invalid GitHub module path: {module_path}")
        
        owner, repo = parts[0], parts[1]
        
        # Download the default branch
        download_url = f"https://github.com/{owner}/{repo}/archive/refs/heads/main.zip"
        
        response = await client.get(download_url, follow_redirects=True)
        
        if not response.is_success:
            # Try master branch
            download_url = f"https://github.com/{owner}/{repo}/archive/refs/heads/master.zip"
            response = await client.get(download_url, follow_redirects=True)
        
        if not response.is_success:
            raise HttpError(f"Failed to download from GitHub: {response.status_code}")
        
        archive_path = temp_dir / "module.zip"
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _extract_module(self, archive_path: Path, extract_dir: Path) -> None:
        """Extract the downloaded module archive."""
        extract_dir.mkdir(parents=True, exist_ok=True)
        
        import asyncio
        
        def extract():
            with zipfile.ZipFile(archive_path, 'r') as zf:
                zf.extractall(extract_dir)
        
        await asyncio.to_thread(extract)
    
    async def _analyze_module(
        self,
        extract_dir: Path,
        module_path: str,
        version_info: Dict[str, Any],
    ) -> str:
        """Analyze a Go module and generate documentation."""
        sections = []
        
        version = version_info.get('Version', 'Unknown')
        
        # Header
        sections.append(f"# {module_path}")
        sections.append(f"\nVersion: {version}")
        sections.append(f"Generated: {datetime.utcnow().isoformat()}")
        
        # Module metadata
        sections.append("\n## Module Information\n")
        sections.append(f"- **Module Path**: {module_path}")
        sections.append(f"- **Version**: {version}")
        
        if 'Time' in version_info:
            sections.append(f"- **Published**: {version_info['Time']}")
        
        # Find the module root
        module_root = extract_dir
        subdirs = list(extract_dir.iterdir())
        if len(subdirs) == 1 and subdirs[0].is_dir():
            module_root = subdirs[0]
        
        # go.mod
        go_mod = await self._read_go_mod(module_root)
        if go_mod:
            sections.append(f"\n## go.mod\n\n```\n{go_mod}\n```")
        
        # README
        readme = await self._find_readme(module_root)
        if readme:
            sections.append(f"\n## README\n\n{readme}")
        
        # File structure
        structure = await self._build_file_tree(module_root)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(module_root)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _read_go_mod(self, module_root: Path) -> Optional[str]:
        """Read the go.mod file."""
        go_mod_path = module_root / "go.mod"
        if go_mod_path.exists():
            try:
                async with aiofiles.open(go_mod_path, 'r', encoding='utf-8') as f:
                    return await f.read()
            except Exception:
                pass
        return None
    
    async def _find_readme(self, module_root: Path) -> Optional[str]:
        """Find and read the README file."""
        readme_names = ['README.md', 'README.rst', 'README.txt', 'README', 'readme.md']
        
        for name in readme_names:
            readme_path = module_root / name
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
        """Collect Go source code from the module."""
        code_sections = []
        file_count = 0
        
        for path in root.rglob('*.go'):
            if file_count >= max_files:
                code_sections.append(f"\n... and more files (limit reached)")
                break
            
            # Skip ignored directories and test files
            if any(skip in path.parts for skip in SKIP_DIRS):
                continue
            if path.name.endswith('_test.go'):
                continue
            
            try:
                async with aiofiles.open(path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = await f.read()
                
                # Skip very large files
                if len(content) > 100000:
                    continue
                
                rel_path = path.relative_to(root)
                code_sections.append(f"\n### {rel_path}\n\n```go\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
