"""
Output organization and indexing utilities.

This module provides functions for organizing and listing output files.
"""

from pathlib import Path
from typing import Dict, List, Optional
from datetime import datetime
import re
import json
import logging

from rich.console import Console
from rich.tree import Tree

logger = logging.getLogger(__name__)
console = Console()


class OutputPaths:
    """
    Structure that manages paths to output directories for different package types.
    """
    
    def __init__(self, base_dir: Path):
        """
        Create a new OutputPaths structure with subdirectories based on the given base directory.
        
        Args:
            base_dir: The base directory to create subdirectories in
        """
        self.base_dir = Path(base_dir)
        self.github_repos_dir = self.base_dir / "github_repos"
        self.github_orgs_dir = self.base_dir / "github_orgs"
        self.pypi_packages_dir = self.base_dir / "pypi_packages"
        self.pypi_profiles_dir = self.base_dir / "pypi_profiles"
        self.crates_dir = self.base_dir / "rust_crates"
        self.npm_packages_dir = self.base_dir / "npm_packages"
        self.go_packages_dir = self.base_dir / "go_packages"
        self.local_repos_dir = self.base_dir / "local_repositories"
        self.index_dir = self.base_dir / "_index"
        self.downloads_dir = self.base_dir / "_downloads"
    
    def ensure_dirs_exist(self) -> None:
        """Ensure all output directories exist, creating them if necessary."""
        for dir_path in [
            self.base_dir,
            self.github_repos_dir,
            self.github_orgs_dir,
            self.pypi_packages_dir,
            self.pypi_profiles_dir,
            self.crates_dir,
            self.npm_packages_dir,
            self.go_packages_dir,
            self.local_repos_dir,
            self.index_dir,
            self.downloads_dir,
        ]:
            dir_path.mkdir(parents=True, exist_ok=True)
    
    def get_dir_for_type(self, package_type: str) -> Path:
        """
        Get the appropriate directory for a given package type.
        
        Args:
            package_type: The type of package
            
        Returns:
            Path to the corresponding directory
        """
        type_map = {
            "github_repo": self.github_repos_dir,
            "github_org": self.github_orgs_dir,
            "pypi": self.pypi_packages_dir,
            "pypi_profile": self.pypi_profiles_dir,
            "crate": self.crates_dir,
            "rust": self.crates_dir,
            "npm": self.npm_packages_dir,
            "go": self.go_packages_dir,
            "local": self.local_repos_dir,
            "local_repo": self.local_repos_dir,
        }
        return type_map.get(package_type.lower(), self.base_dir)


def list_output_files(output_dir: Path) -> None:
    """
    Display a formatted list of all output files organized by category.
    
    Args:
        output_dir: The base output directory to list files from
    """
    console.print("\n[cyan bold]📂 Output Files:[/cyan bold]")
    
    categories = [
        ("Local Repositories", "local_repositories"),
        ("GitHub Repositories", "github_repos"),
        ("GitHub Organizations", "github_orgs"),
        ("PyPI Packages", "pypi_packages"),
        ("PyPI Profiles", "pypi_profiles"),
        ("Go Packages", "go_packages"),
        ("Rust Crates", "rust_crates"),
        ("NPM Packages", "npm_packages"),
    ]
    
    for label, dir_name in categories:
        dir_path = output_dir / dir_name
        
        console.print(f"\n[yellow]{label}[/yellow]")
        
        if not dir_path.exists():
            console.print("  └─ [dim](empty)[/dim]")
            continue
        
        files = list(dir_path.iterdir())
        
        # Sort by modification time (newest first)
        files.sort(key=lambda p: p.stat().st_mtime if p.exists() else 0, reverse=True)
        
        if not files:
            console.print("  └─ [dim](empty)[/dim]")
            continue
        
        for i, file_path in enumerate(files[:10]):  # Limit to 10 files per category
            is_last = i == len(files[:10]) - 1
            prefix = "└─" if is_last else "├─"
            
            filename = file_path.name
            package_name = _extract_package_name(filename)
            
            try:
                size = file_path.stat().st_size
                size_str = _format_file_size(size)
                mtime = datetime.fromtimestamp(file_path.stat().st_mtime)
                time_str = mtime.strftime("%Y-%m-%d %H:%M")
                
                console.print(
                    f"  {prefix} [white bold]{package_name}[/white bold] - "
                    f"[white]{filename}[/white] "
                    f"[dim]({size_str}, {time_str})[/dim]"
                )
            except Exception:
                console.print(f"  {prefix} [white]{filename}[/white]")
        
        if len(files) > 10:
            console.print(f"  ... and {len(files) - 10} more files")


def organize_output(output_dir: Path) -> None:
    """
    Organize output files into appropriate directories.
    
    This function moves any loose files in the output directory
    into their appropriate subdirectories based on their naming.
    
    Args:
        output_dir: The output directory to organize
    """
    output_dir = Path(output_dir)
    
    if not output_dir.exists():
        return
    
    paths = OutputPaths(output_dir)
    paths.ensure_dirs_exist()
    
    # Find files in the root output directory
    for file_path in output_dir.iterdir():
        if file_path.is_dir():
            continue
        
        filename = file_path.name.lower()
        
        # Determine destination based on filename patterns
        dest_dir = None
        
        if 'github' in filename:
            if 'organization' in filename or 'org' in filename:
                dest_dir = paths.github_orgs_dir
            else:
                dest_dir = paths.github_repos_dir
        elif 'pypi' in filename:
            if 'profile' in filename or 'user' in filename:
                dest_dir = paths.pypi_profiles_dir
            else:
                dest_dir = paths.pypi_packages_dir
        elif 'npm' in filename:
            dest_dir = paths.npm_packages_dir
        elif 'crate' in filename or 'rust' in filename:
            dest_dir = paths.crates_dir
        elif 'go' in filename:
            dest_dir = paths.go_packages_dir
        elif 'local' in filename:
            dest_dir = paths.local_repos_dir
        
        if dest_dir:
            try:
                import shutil
                shutil.move(str(file_path), str(dest_dir / file_path.name))
                logger.debug(f"Moved {file_path.name} to {dest_dir}")
            except Exception as e:
                logger.debug(f"Could not move {file_path.name}: {e}")


def generate_index(output_dir: Path) -> None:
    """
    Generate an index file for all output files.
    
    Creates a JSON index with metadata about all processed packages.
    
    Args:
        output_dir: The output directory to index
    """
    output_dir = Path(output_dir)
    paths = OutputPaths(output_dir)
    paths.ensure_dirs_exist()
    
    index = {
        "generated_at": datetime.utcnow().isoformat(),
        "packages": []
    }
    
    categories = {
        "github_repos": "GitHub Repository",
        "github_orgs": "GitHub Organization",
        "pypi_packages": "PyPI Package",
        "pypi_profiles": "PyPI Profile",
        "rust_crates": "Rust Crate",
        "npm_packages": "NPM Package",
        "go_packages": "Go Package",
        "local_repositories": "Local Repository",
    }
    
    for dir_name, category in categories.items():
        dir_path = output_dir / dir_name
        
        if not dir_path.exists():
            continue
        
        for file_path in dir_path.iterdir():
            if not file_path.is_file():
                continue
            
            try:
                stat = file_path.stat()
                package_info = {
                    "name": _extract_package_name(file_path.name),
                    "category": category,
                    "file": str(file_path.relative_to(output_dir)),
                    "size": stat.st_size,
                    "created": datetime.fromtimestamp(stat.st_ctime).isoformat(),
                    "modified": datetime.fromtimestamp(stat.st_mtime).isoformat(),
                }
                index["packages"].append(package_info)
            except Exception as e:
                logger.debug(f"Could not index {file_path}: {e}")
    
    # Sort by modification time (newest first)
    index["packages"].sort(
        key=lambda p: p.get("modified", ""),
        reverse=True
    )
    
    # Write index file
    index_path = paths.index_dir / "index.json"
    with open(index_path, 'w') as f:
        json.dump(index, f, indent=2)
    
    console.print(f"[green]Index generated:[/green] {index_path}")
    console.print(f"[dim]Total packages indexed: {len(index['packages'])}[/dim]")


def _extract_package_name(filename: str) -> str:
    """Extract a human-readable package name from a filename."""
    # Remove timestamp prefix (YYYYMMDD_HHMMSS_)
    name = re.sub(r'^\d{8}_\d{6}_', '', filename)
    
    # Remove _analysis suffix and file extension
    name = re.sub(r'_analysis\.txt$', '', name)
    name = re.sub(r'\.txt$', '', name)
    
    # Replace underscores with spaces for readability
    # But keep certain patterns like version numbers
    name = name.replace('_', ' ')
    
    return name.strip() or filename


def _format_file_size(size_bytes: int) -> str:
    """Format a file size in bytes to a human-readable string."""
    if size_bytes < 1024:
        return f"{size_bytes} B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f} KB"
    elif size_bytes < 1024 * 1024 * 1024:
        return f"{size_bytes / (1024 * 1024):.1f} MB"
    else:
        return f"{size_bytes / (1024 * 1024 * 1024):.1f} GB"
