"""
Path normalization and utilities.
"""

import os
import re
from pathlib import Path
from typing import Union
import logging

logger = logging.getLogger(__name__)


def normalize_url_or_path(input_str: str) -> str:
    """
    Normalize user input that may be a URL or a local path.
    
    Handles:
    - Trailing spaces
    - Quoted paths
    - URL encoding issues
    - Relative path expansion
    
    Args:
        input_str: The user input string
        
    Returns:
        Normalized URL or path string
    """
    # Strip whitespace
    normalized = input_str.strip()
    
    # Remove surrounding quotes if present
    if (normalized.startswith('"') and normalized.endswith('"')) or \
       (normalized.startswith("'") and normalized.endswith("'")):
        normalized = normalized[1:-1]
    
    # Check if it's a URL
    if normalized.startswith(("http://", "https://", "git://", "ssh://")):
        # URL - just clean up whitespace
        return normalized
    
    # Check if it's a pip/npm/cargo command
    if normalized.startswith(("pip ", "npm ", "cargo ")):
        return normalized
    
    # It's a local path - normalize it
    return normalize_user_input_path(normalized)


def normalize_user_input_path(path_str: str) -> str:
    """
    Normalize a user-provided path string.
    
    Args:
        path_str: The path string from user input
        
    Returns:
        Normalized absolute path string
    """
    # Strip whitespace
    path_str = path_str.strip()
    
    # Remove surrounding quotes
    if (path_str.startswith('"') and path_str.endswith('"')) or \
       (path_str.startswith("'") and path_str.endswith("'")):
        path_str = path_str[1:-1]
    
    # Expand user home directory
    path = Path(path_str).expanduser()
    
    # Convert to absolute path
    if not path.is_absolute():
        path = Path.cwd() / path
    
    # Resolve symlinks and normalize
    try:
        path = path.resolve()
    except (OSError, RuntimeError):
        # resolve() can fail on some paths, fall back to absolute()
        path = path.absolute()
    
    return str(path)


def is_local_path(path_str: str) -> bool:
    """
    Check if a string represents a local file or directory path.
    
    Args:
        path_str: The string to check
        
    Returns:
        True if it appears to be a local path
    """
    path_str = path_str.strip()
    
    # Check if it's an absolute path (Unix or Windows)
    if path_str.startswith('/') or path_str.startswith('\\'):
        return True
    
    # Check if it's a relative path
    if path_str.startswith('./') or path_str.startswith('../'):
        return True
    
    # Check if it's a Windows absolute path (e.g., C:\)
    if len(path_str) >= 3 and path_str[1] == ':':
        return True
    
    # Check if the path exists
    if Path(path_str).exists():
        return True
    
    return False


def attempt_permission_elevation(path: Union[str, Path]) -> bool:
    """
    Attempt to gain elevated permissions for a path if needed.
    
    This is a no-op on most systems but can be extended for special cases.
    
    Args:
        path: The path to check
        
    Returns:
        True if elevation was successful or not needed
    """
    path = Path(path)
    
    try:
        # Try to access the path
        if path.is_file():
            path.stat()
        elif path.is_dir():
            list(path.iterdir())
        return True
    except PermissionError:
        logger.warning(f"Permission denied for path: {path}")
        return False
    except Exception as e:
        logger.debug(f"Error accessing path {path}: {e}")
        return False


def show_elevation_hint(path: Union[str, Path]) -> None:
    """
    Show a hint to the user if they need to run with elevated permissions.
    
    Args:
        path: The path that couldn't be accessed
    """
    path = Path(path)
    
    try:
        path.stat()
    except PermissionError:
        import sys
        if sys.platform == "darwin" or sys.platform.startswith("linux"):
            logger.info(f"Hint: You may need to run with 'sudo' to access {path}")
        elif sys.platform == "win32":
            logger.info(f"Hint: You may need to run as Administrator to access {path}")


def sanitize_filename(name: str) -> str:
    """
    Sanitize a string to be used as a filename.
    
    Args:
        name: The string to sanitize
        
    Returns:
        A safe filename string
    """
    # Replace problematic characters
    safe = re.sub(r'[<>:"/\\|?*]', '_', name)
    # Remove leading/trailing whitespace and dots
    safe = safe.strip('. ')
    # Limit length
    if len(safe) > 200:
        safe = safe[:200]
    return safe or "unnamed"


def get_file_extension(path: Union[str, Path]) -> str:
    """
    Get the file extension from a path.
    
    Args:
        path: The file path
        
    Returns:
        The extension including the dot, or empty string
    """
    return Path(path).suffix.lower()


def is_code_file(path: Union[str, Path]) -> bool:
    """
    Check if a file is likely a code file based on its extension.
    
    Args:
        path: The file path
        
    Returns:
        True if it's likely a code file
    """
    CODE_EXTENSIONS = {
        # Systems programming
        '.rs', '.go', '.c', '.cpp', '.h', '.hpp', '.cc', '.cxx',
        # Web development
        '.js', '.ts', '.jsx', '.tsx', '.html', '.css', '.scss', '.sass', '.less',
        # Scripting
        '.py', '.rb', '.php', '.sh', '.bash', '.zsh', '.fish', '.ps1',
        # JVM
        '.java', '.kt', '.scala', '.groovy', '.clj', '.cljs',
        # .NET
        '.cs', '.fs', '.vb',
        # Mobile
        '.swift', '.m', '.mm', '.dart',
        # Functional
        '.hs', '.elm', '.ml', '.ocaml', '.f', '.for', '.f90',
        # Config/Data
        '.json', '.yml', '.yaml', '.toml', '.xml', '.ini', '.cfg', '.conf',
        # Documentation
        '.md', '.rst', '.txt', '.tex', '.org',
        # Build
        '.makefile', '.cmake', '.gradle', '.build', '.bazel',
        # Database
        '.sql', '.sqlite', '.db',
        # Other
        '.r', '.lua', '.vim', '.emacs', '.dockerfile',
    }
    
    ext = get_file_extension(path)
    return ext in CODE_EXTENSIONS


def should_ignore_path(path: Union[str, Path]) -> bool:
    """
    Check if a path should be ignored during processing.
    
    Args:
        path: The path to check
        
    Returns:
        True if the path should be ignored
    """
    IGNORE_PATTERNS = {
        '.git', '.svn', '.hg', '.bzr',
        'node_modules', 'target', 'build', 'dist', 'out',
        '.vscode', '.idea', '.DS_Store', 'Thumbs.db',
        'venv', '.venv', '__pycache__',
        '.pytest_cache', '.coverage', '.tox',
        'vendor', 'deps', 'libs',
    }
    
    path = Path(path)
    
    # Check each component of the path
    for part in path.parts:
        if part in IGNORE_PATTERNS:
            return True
        # Check for hidden files/directories
        if part.startswith('.') and part not in {'.', '..'}:
            # Allow some hidden files
            if part not in {'.gitignore', '.env.example', '.editorconfig'}:
                return True
    
    return False
