"""
Common utilities and helpers for processors.
"""

import re
from pathlib import Path
from typing import Optional
from datetime import datetime

from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn
import aiofiles
import aiofiles.os


def sanitize_filename(name: str) -> str:
    """
    Sanitize a string to be used as a filename.
    
    Args:
        name: The string to sanitize
        
    Returns:
        A safe filename string
    """
    safe = re.sub(r'[<>:"/\\|?*]', '_', name)
    safe = safe.strip('. ')
    if len(safe) > 200:
        safe = safe[:200]
    return safe or "unnamed"


def get_timestamp() -> str:
    """Get a formatted timestamp for file naming."""
    return datetime.utcnow().strftime("%Y%m%d_%H%M%S")


async def save_output_file(
    content: str,
    output_dir: Path,
    filename: str,
    subdirectory: Optional[str] = None,
) -> Path:
    """
    Save content to an output file.
    
    Args:
        content: The content to save
        output_dir: Base output directory
        filename: Name of the file to create
        subdirectory: Optional subdirectory within output_dir
        
    Returns:
        Path to the saved file
    """
    if subdirectory:
        target_dir = output_dir / subdirectory
    else:
        target_dir = output_dir
    
    target_dir.mkdir(parents=True, exist_ok=True)
    
    output_path = target_dir / filename
    
    async with aiofiles.open(output_path, 'w', encoding='utf-8') as f:
        await f.write(content)
    
    return output_path


def create_progress() -> Progress:
    """Create a rich progress bar for display."""
    return Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
    )


def format_file_size(size_bytes: int) -> str:
    """
    Format a file size in bytes to a human-readable string.
    
    Args:
        size_bytes: Size in bytes
        
    Returns:
        Formatted string (e.g., "1.5 MB")
    """
    if size_bytes < 1024:
        return f"{size_bytes} B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f} KB"
    elif size_bytes < 1024 * 1024 * 1024:
        return f"{size_bytes / (1024 * 1024):.1f} MB"
    else:
        return f"{size_bytes / (1024 * 1024 * 1024):.1f} GB"


def detect_language_from_extension(ext: str) -> str:
    """
    Detect programming language from file extension.
    
    Args:
        ext: File extension (with or without leading dot)
        
    Returns:
        Language name
    """
    ext = ext.lower().lstrip('.')
    
    EXTENSION_MAP = {
        'py': 'Python',
        'rs': 'Rust',
        'go': 'Go',
        'js': 'JavaScript',
        'ts': 'TypeScript',
        'jsx': 'JavaScript (React)',
        'tsx': 'TypeScript (React)',
        'java': 'Java',
        'kt': 'Kotlin',
        'scala': 'Scala',
        'rb': 'Ruby',
        'php': 'PHP',
        'c': 'C',
        'cpp': 'C++',
        'cc': 'C++',
        'cxx': 'C++',
        'h': 'C/C++ Header',
        'hpp': 'C++ Header',
        'cs': 'C#',
        'fs': 'F#',
        'swift': 'Swift',
        'm': 'Objective-C',
        'mm': 'Objective-C++',
        'sh': 'Shell',
        'bash': 'Bash',
        'zsh': 'Zsh',
        'ps1': 'PowerShell',
        'sql': 'SQL',
        'html': 'HTML',
        'css': 'CSS',
        'scss': 'SCSS',
        'sass': 'Sass',
        'less': 'Less',
        'json': 'JSON',
        'yaml': 'YAML',
        'yml': 'YAML',
        'toml': 'TOML',
        'xml': 'XML',
        'md': 'Markdown',
        'rst': 'reStructuredText',
        'r': 'R',
        'lua': 'Lua',
        'dart': 'Dart',
        'hs': 'Haskell',
        'elm': 'Elm',
        'ex': 'Elixir',
        'exs': 'Elixir',
        'erl': 'Erlang',
        'clj': 'Clojure',
        'cljs': 'ClojureScript',
        'vue': 'Vue',
        'svelte': 'Svelte',
    }
    
    return EXTENSION_MAP.get(ext, 'Unknown')


async def download_file(url: str, output_path: Path, timeout: int = 120) -> Path:
    """
    Download a file from a URL.
    
    Args:
        url: URL to download from
        output_path: Path to save the downloaded file
        timeout: Request timeout in seconds
        
    Returns:
        Path to the downloaded file
    """
    import httpx
    
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    async with httpx.AsyncClient(timeout=timeout) as client:
        response = await client.get(url, follow_redirects=True)
        response.raise_for_status()
        
        async with aiofiles.open(output_path, 'wb') as f:
            await f.write(response.content)
    
    return output_path
