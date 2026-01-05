"""
Local file/directory processor.

Handles processing of local files and directories.
"""

from pathlib import Path
from typing import Optional, Dict
from datetime import datetime
import logging

import aiofiles
import aiofiles.os

from ..config import Config
from ..error import ProcessorError, ValidationError
from ..utils.path import is_local_path, is_code_file, should_ignore_path
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file, detect_language_from_extension

logger = logging.getLogger(__name__)

LOCAL_REPOS_DIR = "local_repositories"

# Supported file extensions for code analysis
CODE_EXTENSIONS = {
    '.rs', '.go', '.c', '.cpp', '.h', '.hpp', '.cc', '.cxx',
    '.js', '.ts', '.jsx', '.tsx', '.html', '.css', '.scss', '.sass', '.less',
    '.py', '.rb', '.php', '.sh', '.bash', '.zsh', '.fish', '.ps1',
    '.java', '.kt', '.scala', '.groovy', '.clj', '.cljs',
    '.cs', '.fs', '.vb',
    '.swift', '.m', '.mm', '.dart',
    '.hs', '.elm', '.ml', '.ocaml', '.f', '.for', '.f90',
    '.json', '.yml', '.yaml', '.toml', '.xml', '.ini', '.cfg', '.conf',
    '.md', '.rst', '.txt', '.tex', '.org',
    '.makefile', '.cmake', '.gradle', '.build', '.bazel',
    '.sql', '.sqlite', '.db',
    '.r', '.lua', '.vim', '.emacs', '.dockerfile'
}

# Directories to skip
SKIP_DIRS = {
    '.git', '.svn', '.hg', '.bzr',
    'node_modules', 'target', 'build', 'dist', 'out',
    '.vscode', '.idea', '.DS_Store', 'Thumbs.db',
    'venv', '.venv', '__pycache__',
    '.pytest_cache', '.coverage', '.tox',
    'vendor', 'deps', 'libs'
}


class LocalProcessor(PackageProcessor):
    """
    Processor for local directories and files.
    
    Analyzes local codebases and generates documentation.
    """
    
    def name(self) -> str:
        return "Local File/Directory"
    
    def accepts(self, url: str) -> bool:
        return is_local_path(url)
    
    async def validate(self, url: str) -> None:
        """Validate that the path exists."""
        path = Path(url).expanduser().resolve()
        if not path.exists():
            raise ValidationError(f"Path does not exist: {url}")
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process a local file or directory.
        
        Args:
            url: Local path to process
            output_dir: Directory to save output
            config: Application configuration
        """
        input_path = Path(url).expanduser().resolve()
        
        if not input_path.exists():
            raise ValidationError(f"Path does not exist: {url}")
        
        logger.info(f"Processing local path: {input_path}")
        
        # Create output directory
        local_dir = output_dir / LOCAL_REPOS_DIR
        local_dir.mkdir(parents=True, exist_ok=True)
        
        if input_path.is_file():
            analysis = await self._analyze_single_file(input_path)
            name = input_path.name
        else:
            analysis = await self._analyze_directory(input_path, config)
            name = input_path.name
        
        # Generate output
        timestamp = get_timestamp()
        output_filename = f"{timestamp}_{sanitize_filename(name)}_analysis.txt"
        
        await save_output_file(analysis, local_dir, output_filename)
        
        logger.info(f"Analysis saved to {local_dir / output_filename}")
    
    async def _analyze_single_file(self, file_path: Path) -> str:
        """Analyze a single file."""
        sections = []
        
        sections.append(f"# File Analysis: {file_path.name}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        sections.append(f"Path: {file_path}")
        
        # File metadata
        try:
            stat = file_path.stat()
            sections.append(f"\n## File Information\n")
            sections.append(f"- **Size**: {stat.st_size:,} bytes")
            sections.append(f"- **Modified**: {datetime.fromtimestamp(stat.st_mtime).isoformat()}")
        except Exception as e:
            logger.debug(f"Could not get file stats: {e}")
        
        # File type
        ext = file_path.suffix.lower()
        lang = detect_language_from_extension(ext)
        sections.append(f"- **Type**: {lang}")
        
        # Content
        try:
            async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                content = await f.read()
            
            lang_hint = lang.lower().replace(' ', '').replace('/', '')
            sections.append(f"\n## Content\n\n```{lang_hint}\n{content}\n```")
        except Exception as e:
            sections.append(f"\n## Content\n\nError reading file: {e}")
        
        return "\n".join(sections)
    
    async def _analyze_directory(self, dir_path: Path, config: Config) -> str:
        """Analyze a directory."""
        sections = []
        
        sections.append(f"# Directory Analysis: {dir_path.name}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        sections.append(f"Path: {dir_path}")
        
        # Directory statistics
        stats = await self._gather_statistics(dir_path)
        
        sections.append("\n## Statistics\n")
        sections.append(f"- **Total Files**: {stats['file_count']:,}")
        sections.append(f"- **Total Directories**: {stats['dir_count']:,}")
        sections.append(f"- **Total Size**: {stats['total_size']:,} bytes")
        
        # Language breakdown
        if stats['languages']:
            sections.append("\n## Languages\n")
            for lang, count in sorted(stats['languages'].items(), key=lambda x: -x[1]):
                sections.append(f"- {lang}: {count} files")
        
        # README
        readme = await self._find_readme(dir_path)
        if readme:
            sections.append(f"\n## README\n\n{readme}")
        
        # File structure
        structure = await self._build_file_tree(dir_path)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(dir_path, config)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _gather_statistics(self, root: Path) -> Dict:
        """Gather statistics about a directory."""
        stats = {
            'file_count': 0,
            'dir_count': 0,
            'total_size': 0,
            'languages': {},
        }
        
        for path in root.rglob('*'):
            # Skip ignored directories
            if any(skip in path.parts for skip in SKIP_DIRS):
                continue
            
            if path.is_file():
                stats['file_count'] += 1
                try:
                    stats['total_size'] += path.stat().st_size
                except Exception:
                    pass
                
                ext = path.suffix.lower()
                if ext:
                    lang = detect_language_from_extension(ext)
                    if lang != 'Unknown':
                        stats['languages'][lang] = stats['languages'].get(lang, 0) + 1
            elif path.is_dir():
                stats['dir_count'] += 1
        
        return stats
    
    async def _find_readme(self, root: Path) -> Optional[str]:
        """Find and read the README file."""
        readme_names = ['README.md', 'README.rst', 'README.txt', 'README', 'readme.md']
        
        for name in readme_names:
            readme_path = root / name
            if readme_path.exists():
                try:
                    async with aiofiles.open(readme_path, 'r', encoding='utf-8') as f:
                        return await f.read()
                except Exception:
                    continue
        
        return None
    
    async def _build_file_tree(self, root: Path, prefix: str = "", max_depth: int = 5) -> str:
        """Build a string representation of the file tree."""
        lines = []
        
        try:
            items = sorted(root.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower()))
        except PermissionError:
            return f"{prefix}[Permission denied]"
        
        visible_items = [
            item for item in items
            if item.name not in SKIP_DIRS and not item.name.startswith('.')
        ]
        
        for i, item in enumerate(visible_items):
            is_last = i == len(visible_items) - 1
            connector = "└── " if is_last else "├── "
            lines.append(f"{prefix}{connector}{item.name}")
            
            if item.is_dir() and max_depth > 0:
                extension = "    " if is_last else "│   "
                subtree = await self._build_file_tree(item, prefix + extension, max_depth - 1)
                if subtree:
                    lines.append(subtree)
        
        return "\n".join(lines)
    
    async def _collect_source_code(
        self,
        root: Path,
        config: Config,
        max_files: int = 200,
    ) -> str:
        """Collect source code from the directory."""
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
            
            # Skip excluded files from config
            if config.is_excluded_file(path):
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
                
                # Skip binary or non-text content
                if '\x00' in content[:1000]:
                    continue
                
                rel_path = path.relative_to(root)
                lang = detect_language_from_extension(ext).lower().replace(' ', '').replace('/', '')
                
                code_sections.append(f"\n### {rel_path}\n\n```{lang}\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
