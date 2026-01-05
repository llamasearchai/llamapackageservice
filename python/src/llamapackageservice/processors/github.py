"""
GitHub repository processor.

Handles processing of GitHub repositories and organizations.
"""

import os
import re
import zipfile
import tempfile
from pathlib import Path
from typing import Optional, Dict, List, Any
from datetime import datetime
from dataclasses import dataclass
import logging

import httpx
import aiofiles
import aiofiles.os

from ..config import Config
from ..error import ProcessorError, GitHubApiError, ValidationError, HttpError
from .base import PackageProcessor
from .common import sanitize_filename, get_timestamp, save_output_file, detect_language_from_extension

logger = logging.getLogger(__name__)

GITHUB_API_BASE = "https://api.github.com"
GITHUB_BASE = "https://github.com"

# File extensions to process
CODE_EXTENSIONS = {
    '.rs', '.go', '.c', '.cpp', '.h', '.hpp',
    '.js', '.ts', '.jsx', '.tsx', '.html', '.css', '.scss', '.sass',
    '.py', '.rb', '.php', '.sh', '.bash', '.zsh',
    '.java', '.kt', '.scala', '.groovy', '.clj',
    '.cs', '.fs', '.vb',
    '.swift', '.m', '.mm',
    '.json', '.yml', '.yaml', '.toml', '.md'
}

# Directories to skip
SKIP_DIRS = {
    '.git', 'node_modules', 'target', 'build', 'dist', 'out',
    '.vscode', '.idea', '__pycache__', 'venv', '.venv',
    'vendor', 'deps', 'libs'
}


@dataclass
class RepositoryInfo:
    """Information about a GitHub repository."""
    owner: str
    repo: str
    description: Optional[str] = None
    language: Optional[str] = None
    stars: int = 0
    forks: int = 0
    default_branch: str = "main"


@dataclass
class OrganizationInfo:
    """Information about a GitHub organization."""
    name: str
    description: Optional[str] = None
    repo_count: int = 0
    total_stars: int = 0


class GitHubProcessor(PackageProcessor):
    """
    Processor for handling GitHub repositories and organizations.
    
    This processor downloads, extracts, and analyzes GitHub repositories,
    generating comprehensive documentation about the codebase.
    """
    
    def __init__(self):
        self.client: Optional[httpx.AsyncClient] = None
        self._token: Optional[str] = None
    
    def _get_headers(self) -> Dict[str, str]:
        """Get HTTP headers including auth token if available."""
        headers = {
            "Accept": "application/vnd.github.v3+json",
            "User-Agent": "LlamaPackageService/0.1.0",
        }
        if self._token:
            headers["Authorization"] = f"token {self._token}"
        return headers
    
    async def _ensure_client(self, config: Config) -> httpx.AsyncClient:
        """Ensure the HTTP client is initialized."""
        if self.client is None:
            self._token = config.github_token or os.getenv("GITHUB_TOKEN")
            self.client = httpx.AsyncClient(
                timeout=30.0,
                headers=self._get_headers(),
            )
        return self.client
    
    def name(self) -> str:
        return "GitHub Repository"
    
    def accepts(self, url: str) -> bool:
        return "github.com" in url.lower()
    
    async def validate(self, url: str) -> None:
        """Validate the GitHub URL format."""
        parsed = self._parse_github_url(url)
        if parsed is None:
            raise ValidationError(f"Invalid GitHub URL: {url}")
    
    def _parse_github_url(self, url: str) -> Optional[Dict[str, str]]:
        """
        Parse a GitHub URL to extract owner and repo.
        
        Returns:
            Dict with 'owner' and optionally 'repo' keys, or None if invalid
        """
        # Remove trailing slashes and .git suffix
        url = url.rstrip('/').removesuffix('.git')
        
        # Match patterns like:
        # https://github.com/owner/repo
        # https://github.com/owner
        # github.com/owner/repo
        patterns = [
            r'(?:https?://)?github\.com/([^/]+)/([^/]+)',
            r'(?:https?://)?github\.com/([^/]+)/?$',
        ]
        
        for pattern in patterns:
            match = re.match(pattern, url)
            if match:
                groups = match.groups()
                result = {'owner': groups[0]}
                if len(groups) > 1 and groups[1]:
                    result['repo'] = groups[1]
                return result
        
        return None
    
    async def process(self, url: str, output_dir: Path, config: Config) -> None:
        """
        Process a GitHub repository or organization.
        
        Args:
            url: GitHub URL to process
            output_dir: Directory to save output
            config: Application configuration
        """
        client = await self._ensure_client(config)
        parsed = self._parse_github_url(url)
        
        if parsed is None:
            raise ValidationError(f"Invalid GitHub URL: {url}")
        
        owner = parsed['owner']
        repo = parsed.get('repo')
        
        if repo:
            # Process single repository
            await self._process_repository(client, owner, repo, output_dir, config)
        else:
            # Process organization
            await self._process_organization(client, owner, output_dir, config)
    
    async def _process_repository(
        self,
        client: httpx.AsyncClient,
        owner: str,
        repo: str,
        output_dir: Path,
        config: Config,
    ) -> None:
        """Process a single GitHub repository."""
        logger.info(f"Processing repository: {owner}/{repo}")
        
        # Fetch repository info
        repo_info = await self._fetch_repo_info(client, owner, repo)
        
        # Create output directory
        repos_dir = output_dir / "github_repos"
        repos_dir.mkdir(parents=True, exist_ok=True)
        
        # Download and extract repository
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            
            # Download repository archive
            archive_path = await self._download_repo_archive(
                client, owner, repo, repo_info.default_branch, temp_path
            )
            
            # Extract archive
            extract_dir = temp_path / "extracted"
            await self._extract_archive(archive_path, extract_dir)
            
            # Find the actual repo directory (usually has a suffix like -main or -master)
            repo_dirs = list(extract_dir.iterdir())
            if repo_dirs:
                repo_root = repo_dirs[0]
            else:
                repo_root = extract_dir
            
            # Analyze the repository
            analysis = await self._analyze_repository(repo_root, repo_info)
            
            # Generate output
            timestamp = get_timestamp()
            output_filename = f"{timestamp}_{sanitize_filename(owner)}_{sanitize_filename(repo)}_analysis.txt"
            
            await save_output_file(analysis, repos_dir, output_filename)
            
            logger.info(f"Repository analysis saved to {repos_dir / output_filename}")
    
    async def _process_organization(
        self,
        client: httpx.AsyncClient,
        org: str,
        output_dir: Path,
        config: Config,
    ) -> None:
        """Process a GitHub organization."""
        logger.info(f"Processing organization: {org}")
        
        # Fetch organization info and repositories
        org_info = await self._fetch_org_info(client, org)
        repos = await self._fetch_org_repos(client, org)
        
        # Create output directory
        orgs_dir = output_dir / "github_orgs" / sanitize_filename(org)
        orgs_dir.mkdir(parents=True, exist_ok=True)
        
        # Generate organization overview
        overview = self._generate_org_overview(org_info, repos)
        timestamp = get_timestamp()
        
        await save_output_file(
            overview,
            orgs_dir,
            f"{timestamp}_{sanitize_filename(org)}_organization_overview.txt"
        )
        
        # Process top repositories (limit to avoid excessive API calls)
        max_repos = 10
        sorted_repos = sorted(repos, key=lambda r: r.get('stargazers_count', 0), reverse=True)
        
        for repo_data in sorted_repos[:max_repos]:
            try:
                await self._process_repository(
                    client, org, repo_data['name'], orgs_dir, config
                )
            except Exception as e:
                logger.warning(f"Failed to process repo {repo_data['name']}: {e}")
    
    async def _fetch_repo_info(
        self,
        client: httpx.AsyncClient,
        owner: str,
        repo: str,
    ) -> RepositoryInfo:
        """Fetch repository information from GitHub API."""
        url = f"{GITHUB_API_BASE}/repos/{owner}/{repo}"
        
        response = await client.get(url)
        
        if response.status_code == 404:
            raise ValidationError(f"Repository not found: {owner}/{repo}")
        elif response.status_code == 403:
            raise GitHubApiError("Rate limit exceeded or authentication required")
        elif not response.is_success:
            raise HttpError(f"Failed to fetch repository info: {response.status_code}")
        
        data = response.json()
        
        return RepositoryInfo(
            owner=owner,
            repo=repo,
            description=data.get('description'),
            language=data.get('language'),
            stars=data.get('stargazers_count', 0),
            forks=data.get('forks_count', 0),
            default_branch=data.get('default_branch', 'main'),
        )
    
    async def _fetch_org_info(
        self,
        client: httpx.AsyncClient,
        org: str,
    ) -> OrganizationInfo:
        """Fetch organization information from GitHub API."""
        url = f"{GITHUB_API_BASE}/orgs/{org}"
        
        response = await client.get(url)
        
        if response.status_code == 404:
            raise ValidationError(f"Organization not found: {org}")
        elif not response.is_success:
            raise HttpError(f"Failed to fetch organization info: {response.status_code}")
        
        data = response.json()
        
        return OrganizationInfo(
            name=org,
            description=data.get('description'),
            repo_count=data.get('public_repos', 0),
        )
    
    async def _fetch_org_repos(
        self,
        client: httpx.AsyncClient,
        org: str,
    ) -> List[Dict[str, Any]]:
        """Fetch repositories for an organization."""
        repos = []
        page = 1
        per_page = 100
        
        while True:
            url = f"{GITHUB_API_BASE}/orgs/{org}/repos"
            params = {'page': page, 'per_page': per_page, 'type': 'public'}
            
            response = await client.get(url, params=params)
            
            if not response.is_success:
                break
            
            page_repos = response.json()
            if not page_repos:
                break
            
            repos.extend(page_repos)
            page += 1
            
            # Limit to avoid excessive API calls
            if len(repos) >= 100:
                break
        
        return repos
    
    async def _download_repo_archive(
        self,
        client: httpx.AsyncClient,
        owner: str,
        repo: str,
        branch: str,
        temp_dir: Path,
    ) -> Path:
        """Download repository archive as ZIP."""
        url = f"{GITHUB_BASE}/{owner}/{repo}/archive/refs/heads/{branch}.zip"
        
        response = await client.get(url, follow_redirects=True)
        
        if not response.is_success:
            raise HttpError(f"Failed to download repository archive: {response.status_code}")
        
        archive_path = temp_dir / f"{repo}.zip"
        
        async with aiofiles.open(archive_path, 'wb') as f:
            await f.write(response.content)
        
        return archive_path
    
    async def _extract_archive(self, archive_path: Path, extract_dir: Path) -> None:
        """Extract a ZIP archive."""
        extract_dir.mkdir(parents=True, exist_ok=True)
        
        # Use zipfile in a thread to avoid blocking
        import asyncio
        
        def extract():
            with zipfile.ZipFile(archive_path, 'r') as zf:
                zf.extractall(extract_dir)
        
        await asyncio.to_thread(extract)
    
    async def _analyze_repository(
        self,
        repo_root: Path,
        repo_info: RepositoryInfo,
    ) -> str:
        """Analyze a repository and generate documentation."""
        sections = []
        
        # Header
        sections.append(f"# {repo_info.owner}/{repo_info.repo}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        sections.append(f"Stars: {repo_info.stars} | Forks: {repo_info.forks}")
        if repo_info.language:
            sections.append(f"Primary Language: {repo_info.language}")
        if repo_info.description:
            sections.append(f"\n## Description\n\n{repo_info.description}")
        
        # README
        readme = await self._find_readme(repo_root)
        if readme:
            sections.append(f"\n## README\n\n{readme}")
        
        # File structure
        structure = await self._build_file_tree(repo_root)
        sections.append(f"\n## File Structure\n\n```\n{structure}\n```")
        
        # Language statistics
        lang_stats = await self._analyze_languages(repo_root)
        if lang_stats:
            sections.append("\n## Language Statistics\n")
            for lang, count in sorted(lang_stats.items(), key=lambda x: -x[1]):
                sections.append(f"- {lang}: {count} files")
        
        # Source code
        sections.append("\n## Source Code\n")
        code_content = await self._collect_source_code(repo_root)
        sections.append(code_content)
        
        return "\n".join(sections)
    
    async def _find_readme(self, repo_root: Path) -> Optional[str]:
        """Find and read the README file."""
        readme_names = ['README.md', 'README.rst', 'README.txt', 'README', 'readme.md']
        
        for name in readme_names:
            readme_path = repo_root / name
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
    
    async def _analyze_languages(self, root: Path) -> Dict[str, int]:
        """Analyze programming languages used in the repository."""
        stats: Dict[str, int] = {}
        
        for path in root.rglob('*'):
            if path.is_file():
                # Skip ignored directories
                if any(skip in path.parts for skip in SKIP_DIRS):
                    continue
                
                ext = path.suffix.lower()
                if ext in CODE_EXTENSIONS:
                    lang = detect_language_from_extension(ext)
                    stats[lang] = stats.get(lang, 0) + 1
        
        return stats
    
    async def _collect_source_code(self, root: Path, max_files: int = 100) -> str:
        """Collect source code from the repository."""
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
                lang = detect_language_from_extension(ext).lower()
                
                code_sections.append(f"\n### {rel_path}\n\n```{lang}\n{content}\n```")
                file_count += 1
                
            except Exception as e:
                logger.debug(f"Could not read file {path}: {e}")
        
        return "\n".join(code_sections)
    
    def _generate_org_overview(
        self,
        org_info: OrganizationInfo,
        repos: List[Dict[str, Any]],
    ) -> str:
        """Generate an overview of a GitHub organization."""
        sections = []
        
        sections.append(f"# GitHub Organization: {org_info.name}")
        sections.append(f"\nGenerated: {datetime.utcnow().isoformat()}")
        
        if org_info.description:
            sections.append(f"\n## Description\n\n{org_info.description}")
        
        sections.append(f"\n## Statistics\n")
        sections.append(f"- Public Repositories: {org_info.repo_count}")
        
        total_stars = sum(r.get('stargazers_count', 0) for r in repos)
        sections.append(f"- Total Stars: {total_stars}")
        
        sections.append("\n## Top Repositories\n")
        sorted_repos = sorted(repos, key=lambda r: r.get('stargazers_count', 0), reverse=True)
        
        for repo in sorted_repos[:20]:
            name = repo.get('name', 'Unknown')
            stars = repo.get('stargazers_count', 0)
            language = repo.get('language', 'Unknown')
            desc = repo.get('description', '')[:100] if repo.get('description') else ''
            
            sections.append(f"### {name}")
            sections.append(f"- Stars: {stars}")
            sections.append(f"- Language: {language}")
            if desc:
                sections.append(f"- Description: {desc}")
            sections.append("")
        
        return "\n".join(sections)
