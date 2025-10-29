"""
GitHub Repository Manager with AI Integration
"""
import asyncio
import json
import subprocess
from pathlib import Path
from typing import Dict, List, Optional, Any
import git
from github import Github
import aiofiles
import logging
from datetime import datetime

from .mcp_client import MCPClient
from .ollama_interface import OllamaInterface
from ..utils.config import ConfigManager

logger = logging.getLogger(__name__)


class GitHubRepoManager:
    def __init__(self, config_path: str = "config/github_config.yaml"):
        self.config = ConfigManager(config_path)
        self.github = Github(self.config.get('github_token'))
        self.mcp_client = MCPClient()
        self.ollama = OllamaInterface()
        self.local_repos_path = Path(self.config.get('local_repos_path', '~/Development')).expanduser()
        self.repo_cache = {}
        
    async def initialize(self):
        """Initialize all components"""
        await self.mcp_client.connect_servers(self.config.get('mcp_servers', []))
        await self.ollama.initialize_models()
        self.scan_local_repositories()
        logger.info("GitHub Repository Manager initialized")
        
    def scan_local_repositories(self) -> List[Dict]:
        """Scan and catalog all local repositories"""
        repos = []
        if not self.local_repos_path.exists():
            self.local_repos_path.mkdir(parents=True, exist_ok=True)
            
        for repo_path in self.local_repos_path.rglob('.git'):
            if repo_path.is_dir():
                try:
                    repo = git.Repo(repo_path.parent)
                    repo_info = {
                        'name': repo_path.parent.name,
                        'path': str(repo_path.parent),
                        'remote_url': self.get_remote_url(repo),
                        'current_branch': repo.active_branch.name if repo.active_branch else 'detached',
                        'status': self.get_repo_status(repo),
                        'last_commit': self.get_last_commit_info(repo)
                    }
                    repos.append(repo_info)
                    self.repo_cache[repo_info['name']] = repo_info
                except Exception as e:
                    logger.warning(f"Failed to scan repo {repo_path.parent}: {e}")
        return repos
    
    def get_remote_url(self, repo: git.Repo) -> Optional[str]:
        """Get remote URL for repository"""
        try:
            if repo.remotes:
                return repo.remotes[0].url
        except:
            pass
        return None
    
    def get_repo_status(self, repo: git.Repo) -> Dict[str, Any]:
        """Get repository status"""
        try:
            return {
                'is_dirty': repo.is_dirty(),
                'untracked_files': len(repo.untracked_files),
                'modified_files': len([item.a_path for item in repo.index.diff(None)]),
                'staged_files': len([item.a_path for item in repo.index.diff('HEAD')])
            }
        except:
            return {'error': 'Unable to get status'}
    
    def get_last_commit_info(self, repo: git.Repo) -> Dict[str, Any]:
        """Get last commit information"""
        try:
            commit = repo.head.commit
            return {
                'sha': commit.hexsha[:7],
                'message': commit.message.strip(),
                'author': str(commit.author),
                'date': commit.committed_datetime.isoformat()
            }
        except:
            return {'error': 'No commits'}
    
    async def ai_analyze_repository(self, repo_path: str) -> Dict:
        """Use Ollama to analyze repository structure and suggest improvements"""
        analysis_prompt = f"""
        Analyze the repository at {repo_path}. Provide:
        1. Project structure assessment
        2. Code quality insights
        3. Suggested improvements
        4. Security considerations
        5. Documentation status
        6. Testing coverage analysis
        7. Dependency health check
        """
        
        # Gather repository data
        repo_data = await self.gather_repo_data(repo_path)
        
        # Use vision model if screenshots/diagrams present
        if self.has_visual_content(repo_path):
            return await self.ollama.analyze_with_vision(repo_path, analysis_prompt, repo_data)
        else:
            return await self.ollama.analyze_code(repo_path, analysis_prompt, repo_data)
    
    async def gather_repo_data(self, repo_path: str) -> Dict[str, Any]:
        """Gather comprehensive repository data"""
        data = {
            'files': [],
            'structure': {},
            'languages': {},
            'dependencies': {},
            'tests': [],
            'docs': []
        }
        
        path = Path(repo_path)
        
        # File analysis
        for file_path in path.rglob('*'):
            if file_path.is_file() and not any(part.startswith('.') for part in file_path.parts):
                rel_path = file_path.relative_to(path)
                ext = file_path.suffix
                
                data['files'].append(str(rel_path))
                
                if ext:
                    data['languages'][ext] = data['languages'].get(ext, 0) + 1
                
                # Identify tests
                if any(pattern in str(rel_path).lower() for pattern in ['test', 'spec']):
                    data['tests'].append(str(rel_path))
                
                # Identify docs
                if ext in ['.md', '.rst', '.txt'] or 'doc' in str(rel_path).lower():
                    data['docs'].append(str(rel_path))
        
        # Dependency files
        dep_files = {
            'package.json': 'npm',
            'requirements.txt': 'pip',
            'Cargo.toml': 'cargo',
            'go.mod': 'go',
            'pom.xml': 'maven',
            'build.gradle': 'gradle'
        }
        
        for dep_file, dep_type in dep_files.items():
            if (path / dep_file).exists():
                data['dependencies'][dep_type] = dep_file
        
        return data
    
    def has_visual_content(self, repo_path: str) -> bool:
        """Check if repository contains visual content"""
        visual_extensions = {'.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp'}
        path = Path(repo_path)
        
        for ext in visual_extensions:
            if list(path.rglob(f'*{ext}')):
                return True
        return False
    
    async def create_repository(self, name: str, description: str = "", private: bool = False) -> Dict:
        """Create a new GitHub repository"""
        try:
            user = self.github.get_user()
            repo = user.create_repo(
                name=name,
                description=description,
                private=private,
                auto_init=True
            )
            
            # Clone locally
            local_path = self.local_repos_path / name
            git.Repo.clone_from(repo.clone_url, str(local_path))
            
            return {
                'success': True,
                'name': repo.name,
                'url': repo.html_url,
                'local_path': str(local_path)
            }
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    async def sync_repository(self, repo_name: str) -> Dict:
        """Sync local repository with remote"""
        try:
            repo_info = self.repo_cache.get(repo_name)
            if not repo_info:
                return {'success': False, 'error': 'Repository not found'}
            
            repo = git.Repo(repo_info['path'])
            origin = repo.remote('origin')
            
            # Fetch latest changes
            origin.fetch()
            
            # Pull if no local changes
            if not repo.is_dirty():
                origin.pull()
                return {'success': True, 'action': 'pulled'}
            else:
                return {
                    'success': False, 
                    'error': 'Local changes present',
                    'modified_files': [item.a_path for item in repo.index.diff(None)]
                }
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    async def analyze_issues(self, repo_name: str) -> Dict:
        """Analyze repository issues using AI"""
        try:
            repo = self.github.get_repo(repo_name)
            issues = list(repo.get_issues(state='open'))
            
            analysis = {
                'total_open': len(issues),
                'by_label': {},
                'priority_issues': [],
                'stale_issues': [],
                'suggested_actions': []
            }
            
            # Categorize issues
            for issue in issues:
                for label in issue.labels:
                    if label.name not in analysis['by_label']:
                        analysis['by_label'][label.name] = 0
                    analysis['by_label'][label.name] += 1
                
                # Check for priority
                if any(label.name in ['critical', 'high-priority', 'bug'] for label in issue.labels):
                    analysis['priority_issues'].append({
                        'number': issue.number,
                        'title': issue.title,
                        'created_at': issue.created_at.isoformat()
                    })
                
                # Check for stale issues (older than 30 days)
                if (datetime.utcnow() - issue.created_at).days > 30:
                    analysis['stale_issues'].append({
                        'number': issue.number,
                        'title': issue.title,
                        'days_old': (datetime.utcnow() - issue.created_at).days
                    })
            
            # AI analysis for suggestions
            if issues:
                ai_prompt = f"""
                Analyze these GitHub issues and suggest actions:
                {json.dumps([{'title': i.title, 'body': i.body} for i in issues[:10]], indent=2)}
                
                Provide:
                1. Common themes
                2. Priority order
                3. Suggested resolutions
                """
                
                ai_analysis = await self.ollama.analyze_text(ai_prompt)
                analysis['ai_insights'] = ai_analysis
            
            return analysis
            
        except Exception as e:
            return {'error': str(e)}
    
    async def batch_operation(self, operation: str, repo_names: List[str], **kwargs) -> Dict:
        """Perform batch operations on multiple repositories"""
        results = {}
        
        operations = {
            'sync': self.sync_repository,
            'analyze': self.ai_analyze_repository,
            'backup': self.backup_repository,
            'update_deps': self.update_dependencies
        }
        
        if operation not in operations:
            return {'error': f'Unknown operation: {operation}'}
        
        op_func = operations[operation]
        
        for repo_name in repo_names:
            try:
                results[repo_name] = await op_func(repo_name, **kwargs)
            except Exception as e:
                results[repo_name] = {'error': str(e)}
        
        return results
    
    async def backup_repository(self, repo_name: str) -> Dict:
        """Backup repository to specified location"""
        try:
            repo_info = self.repo_cache.get(repo_name)
            if not repo_info:
                return {'success': False, 'error': 'Repository not found'}
            
            backup_dir = Path(self.config.get('backup_path', '~/Backups/repos')).expanduser()
            backup_dir.mkdir(parents=True, exist_ok=True)
            
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            backup_path = backup_dir / f"{repo_name}_{timestamp}.tar.gz"
            
            # Create backup
            subprocess.run([
                'tar', '-czf', str(backup_path), 
                '-C', str(Path(repo_info['path']).parent), 
                Path(repo_info['path']).name
            ], check=True)
            
            return {
                'success': True,
                'backup_path': str(backup_path),
                'size': backup_path.stat().st_size
            }
            
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    async def update_dependencies(self, repo_name: str) -> Dict:
        """Update repository dependencies"""
        try:
            repo_info = self.repo_cache.get(repo_name)
            if not repo_info:
                return {'success': False, 'error': 'Repository not found'}
            
            repo_path = Path(repo_info['path'])
            results = {}
            
            # NPM update
            if (repo_path / 'package.json').exists():
                result = subprocess.run(
                    ['npm', 'update'], 
                    cwd=repo_path, 
                    capture_output=True, 
                    text=True
                )
                results['npm'] = {
                    'success': result.returncode == 0,
                    'output': result.stdout
                }
            
            # Python update
            if (repo_path / 'requirements.txt').exists():
                result = subprocess.run(
                    ['pip', 'install', '--upgrade', '-r', 'requirements.txt'],
                    cwd=repo_path,
                    capture_output=True,
                    text=True
                )
                results['pip'] = {
                    'success': result.returncode == 0,
                    'output': result.stdout
                }
            
            # Cargo update
            if (repo_path / 'Cargo.toml').exists():
                result = subprocess.run(
                    ['cargo', 'update'],
                    cwd=repo_path,
                    capture_output=True,
                    text=True
                )
                results['cargo'] = {
                    'success': result.returncode == 0,
                    'output': result.stdout
                }
            
            return {'success': True, 'updates': results}
            
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    async def cleanup(self):
        """Cleanup resources"""
        await self.mcp_client.disconnect()
        await self.ollama.cleanup()