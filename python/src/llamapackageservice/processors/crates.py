"""
Rust crates processor.

Handles processing of Rust packages from crates.io.
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
from ..error import ProcessorError, ValidationError, HttpError
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file

logger = logging.getLogger(__name__)

CRATES_API = "https://crates.io/api/v1"
CRATES_DL = "https://crates.io/api/v1/crates"

# File extensions to process
CODE_EXTENSIONS = {'.rs', '.toml'}

# Directories to skip
SKIP_DIRS = {'target', '.git', '.cargo'}


class CratesProcessor(PackageProcessor):
    """
    Processor for handling Rust crates from crates.io.
    
    This processor can extract information from crates.io URLs or cargo commands,
    download the crates, and generate comprehensive documentation.
    """
    
    def __init__(self):
        self.client: Optional[httpx.AsyncClient] = None
    
    async def _ensure_client(self) -> httpx.AsyncClient:
        """Ensure the HTTP client is initialized."""
        if self.client is None:
            self.client = httpx.AsyncClient(
                timeout=120.0,
                headers={
                    "User-Agent": "LlamaPackageService/0.1.0 (https://llamasearch.ai)"
                }
            )
        return self.client
    
    def name(self) -> str:
        return "Rust Crate"
    
    def accepts(self, url: str) -> bool:
        url_lower = url.lower()
        return (
            "crates.io" in url_lower or
            "docs.rs" in url_lower or
            url.startswith("cargo add ")
        )
    
    async def validate(self, url: str) -> None:
        """Validate the crates.io URL or cargo command."""
        crate_name = self._extract_crate_name(url)
        if not crate_name:
            raise ValidationError(f"Could not extract crate name from: {url}")
        
        # Validate crate exists
        client = await self._ensure_client()
        api_url = f"{CRATES_API}/crates/{crate_name}"
        
        response = await client.get(api_url)
        if response.status_code == 404:
            raise ValidationError(f"Crate not found: {crate_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to validate crate: {response.status_code}")
    
    def _extract_crate_name(self, url: str) -> Optional[str]:
        """
        Extract crate name from various input formats.
        
        Supports:
        - https://crates.io/crates/crate-name
        - https://docs.rs/crate-name
        - cargo add crate-name
        """
        # From crates.io URL
        match = re.search(r'crates\.io/crates/([^/\s]+)', url)
        if match:
            return match.group(1)
        
        # From docs.rs URL
        match = re.search(r'docs\.rs/([^/\s]+)', url)
        if match:
            return match.group(1)
        
        # From cargo command
        match = re.search(r'cargo\s+add\s+([^\s]+)', url)
        if match:
            return match.group(1)
        
        # Direct crate name
        if not ' ' in url and not '/' in url:
            return url.strip()
        
        return None
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process a Rust crate.
        
        Args:
            url: crates.io URL or cargo command
            output_dir: Directory to save output
            config: Application configuration
        """
        client = await self._ensure_client()
        crate_name = self._extract_crate_name(url)
        
        if not crate_name:
            raise ValidationError(f"Could not extract crate name from: {url}")
        
        logger.info(f"Processing Rust crate: {crate_name}")
        
        # Fetch crate info
        crate_info = await self._fetch_crate_info(client, crate_name)
        
        # Create output directory
        crates_dir = output_dir / "rust_crates"
        crates_dir.mkdir(parents=True, exist_ok=True)
        
        # Download and extract crate
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            
            # Download crate
            version = crate_info.get('crate', {}).get('max_version', 'latest')
            archive_path = await self._download_crate(client, crate_name, version, temp_path)
            
            # Extract crate
            extract_dir = temp_path / "extracted"
            await self._extract_crate(archive_path, extract_dir)
            
            # Analyze the crate
            analysis = await self._analyze_crate(extract_dir, crate_info)
            
            # Generate output
            timestamp = get_timestamp()
            output_filename = f"{timestamp}_{sanitize_filename(crate_name)}_{version}_analysis.txt"
            
            await save_output_file(analysis, crates_dir, output_filename)
            
            logger.info(f"Crate analysis saved to {crates_dir / output_filename}")
    
    async def _fetch_crate_info(
        self,
        client: httpx.AsyncClient,
        crate_name: str,
    ) -> Dict[str, Any]:
        """Fetch crate information from crates.io API."""
        url = f"{CRATES_API}/crates/{crate_name}"
        
        response = await client.get(url)
        
        if response.status_code == 404:
            raise ValidationError(f"Crate not found: {crate_name}")
        elif not response.is_success:
            raise HttpError(f"Failed to fetch crate info: {response.status_code}")
        
        return response.json()
    
    async def _download_crate(
        self,
        client: httpx.AsyncClient,
        crate_name: str,
        version: str,
        temp_dir: Path,
    ) -> Path:
        """Download the crate archive."""
        download_url = f"{CRATES_DL}/{crate_name}/{version}/download"
        
        logger.info(f"Downloading crate from {download_url}")
        
        response = await client.get(download_url, follow_redirects=True)
        
        if not response.is_success:
            raise HttpError(f"Failed to download crate: {response.status_code}")
        
        archive_path = temp_dir / f"{crate_name}-{version}.crate"
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _extract_crate(self, archive_path: Path, extract_dir: Path) -> None:
        """Extract the downloaded crate archive (tar.gz format)."""
        extract_dir.mkdir(parents=True, exist_ok=True)
        
        import asyncio
        
        def extract():
            with tarfile.open(archive_path, 'r:gz') as tf:
                tf.extractall(extract_dir)
        
        await asyncio.to_thread(extract)
    
    async def _analyze_crate(
        self,
        extract_dir: Path,
        crate_info: Dict[str, Any],
    ) -> str:
        """Analyze a crate and generate documentation."""
        sections = []
        
        crate = crate_info.get('crate', {})
        versions = crate_info.get('versions', [])
        
        # Header
        name = crate.get('name', 'Unknown')
        version = crate.get('max_version', 'Unknown')
        
        sections.append(f"# {name} v{version}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        
        # Crate metadata
        sections.append("\n## Crate Information\n")
        sections.append(f"- **Name**: {name}")
        sections.append(f"- **Version**: {version}")
        sections.append(f"- **Description**: {crate.get('description', 'N/A')}")
        sections.append(f"- **Downloads**: {crate.get('downloads', 0):,}")
        sections.append(f"- **Recent Downloads**: {crate.get('recent_downloads', 0):,}")
        
        if crate.get('homepage'):
            sections.append(f"- **Homepage**: {crate['homepage']}")
        if crate.get('repository'):
            sections.append(f"- **Repository**: {crate['repository']}")
        if crate.get('documentation'):
            sections.append(f"- **Documentation**: {crate['documentation']}")
        
        # Categories and keywords
        categories = crate.get('categories', [])
        keywords = crate.get('keywords', [])
        
        if keywords:
            sections.append(f"- **Keywords**: {', '.join(keywords)}")
        if categories:
            sections.append(f"- **Categories**: {', '.join(categories)}")
        
        # Version history
        if versions:
            sections.append("\n## Version History\n")
            for v in versions[:10]:  # Show last 10 versions
                v_num = v.get('num', 'Unknown')
                v_date = v.get('created_at', 'Unknown')[:10] if v.get('created_at') else 'Unknown'
                sections.append(f"- {v_num} ({v_date})")
        
        # Find the crate root
        crate_root = extract_dir
        subdirs = list(extract_dir.iterdir())
        if len(subdirs) == 1 and subdirs[0].is_dir():
            crate_root = subdirs[0]
        
        # README
        readme = await self._find_readme(crate_root)
        if readme:
            sections.append(f"\n## README\n\n{readme}")
        
        # Cargo.toml
        cargo_toml = await self._read_cargo_toml(crate_root)
        if cargo_toml:
            sections.append(f"\n## Cargo.toml\n\n```toml\n{cargo_toml}\n```")
        
        # File structure
        structure = await self._build_file_tree(crate_root)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(crate_root)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _find_readme(self, crate_root: Path) -> Optional[str]:
        """Find and read the README file."""
        readme_names = ['README.md', 'README.rst', 'README.txt', 'README', 'readme.md']
        
        for name in readme_names:
            readme_path = crate_root / name
            if readme_path.exists():
                try:
                    async with aiofiles.open(readme_path, 'r', encoding='utf-8') as f:
                        return await f.read()
                except Exception:
                    continue
        
        return None
    
    async def _read_cargo_toml(self, crate_root: Path) -> Optional[str]:
        """Read the Cargo.toml file."""
        cargo_path = crate_root / "Cargo.toml"
        if cargo_path.exists():
            try:
                async with aiofiles.open(cargo_path, 'r', encoding='utf-8') as f:
                    return await f.read()
            except Exception:
                pass
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
        """Collect Rust source code from the crate."""
        code_sections = []
        file_count = 0
        
        for path in root.rglob('*.rs'):
            if file_count >= max_files:
                code_sections.append(f"\n... and more files (limit reached)")
                break
            
            # Skip ignored directories
            if any(skip in path.parts for skip in SKIP_DIRS):
                continue
            
            try:
                async with aiofiles.open(path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = await f.read()
                
                # Skip very large files
                if len(content) > 100000:
                    continue
                
                rel_path = path.relative_to(root)
                code_sections.append(f"\n### {rel_path}\n\n```rust\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
