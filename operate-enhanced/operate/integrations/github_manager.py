"""GitHub integration for repository management and automation."""
import asyncio
import json
import os
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Set, Tuple
import logging

from github import Github, GithubException
from github.Repository import Repository
from github.PullRequest import PullRequest
from github.Issue import Issue
import git
import aiofiles
import httpx

from ..interfaces import Action, ActionType, OperationResult, OperationStatus
from ..core.orchestrator import Operation


logger = logging.getLogger(__name__)


@dataclass
class CodeAnalysis:
    """Results of code analysis."""
    languages: Dict[str, int]
    dependencies: Dict[str, List[str]]
    structure: Dict[str, Any]
    tests: List[str]
    coverage: Optional[float] = None
    complexity: Optional[Dict[str, float]] = None


@dataclass
class PRReview:
    """Pull request review data."""
    pr_number: int
    title: str
    description: str
    files_changed: List[str]
    additions: int
    deletions: int
    suggestions: List[Dict[str, str]]
    approval_status: str
    comments: List[str]


class GitHubManager:
    """Manages GitHub operations and repository automation."""
    
    def __init__(self, token: str, ai_model=None):
        self.github = Github(token)
        self.ai_model = ai_model
        self.local_repos: Dict[str, git.Repo] = {}
        self.monitors: Dict[str, asyncio.Task] = {}
        self._client = httpx.AsyncClient()
        
    async def initialize(self):
        """Initialize the GitHub manager."""
        user = self.github.get_user()
        logger.info(f"GitHub manager initialized for user: {user.login}")
        
    async def clone_repository(self, repo_name: str, local_path: Optional[str] = None) -> git.Repo:
        """Clone a repository locally."""
        try:
            repo = self.github.get_repo(repo_name)
            
            if local_path is None:
                local_path = f"./repos/{repo_name.replace('/', '_')}"
            
            Path(local_path).parent.mkdir(parents=True, exist_ok=True)
            
            if os.path.exists(local_path):
                # Update existing repo
                local_repo = git.Repo(local_path)
                origin = local_repo.remote("origin")
                origin.pull()
            else:
                # Clone new repo
                local_repo = git.Repo.clone_from(repo.clone_url, local_path)
            
            self.local_repos[repo_name] = local_repo
            return local_repo
            
        except Exception as e:
            logger.error(f"Failed to clone repository {repo_name}: {str(e)}")
            raise
    
    async def analyze_codebase(self, repo_path: str) -> CodeAnalysis:
        """Analyze a codebase for structure and metrics."""
        analysis = CodeAnalysis(
            languages={},
            dependencies={},
            structure={},
            tests=[]
        )
        
        # Language detection
        for root, dirs, files in os.walk(repo_path):
            # Skip hidden and vendor directories
            dirs[:] = [d for d in dirs if not d.startswith('.') and d not in ['vendor', 'node_modules']]
            
            for file in files:
                ext = Path(file).suffix
                if ext:
                    analysis.languages[ext] = analysis.languages.get(ext, 0) + 1
        
        # Dependency detection
        dependency_files = {
            'package.json': self._parse_npm_dependencies,
            'requirements.txt': self._parse_python_requirements,
            'Cargo.toml': self._parse_cargo_dependencies,
            'go.mod': self._parse_go_dependencies,
            'pom.xml': self._parse_maven_dependencies,
        }
        
        for dep_file, parser in dependency_files.items():
            dep_path = Path(repo_path) / dep_file
            if dep_path.exists():
                deps = await parser(dep_path)
                if deps:
                    analysis.dependencies[dep_file] = deps
        
        # Find test files
        test_patterns = ['test_*.py', '*_test.py', '*.test.js', '*.spec.js', '*_test.go']
        for pattern in test_patterns:
            tests = list(Path(repo_path).rglob(pattern))
            analysis.tests.extend([str(t.relative_to(repo_path)) for t in tests])
        
        # Directory structure
        analysis.structure = await self._analyze_structure(repo_path)
        
        return analysis
    
    async def create_pull_request(
        self,
        repo_name: str,
        title: str,
        body: str,
        head_branch: str,
        base_branch: str = "main"
    ) -> PullRequest:
        """Create a pull request."""
        try:
            repo = self.github.get_repo(repo_name)
            pr = repo.create_pull(
                title=title,
                body=body,
                head=head_branch,
                base=base_branch
            )
            
            logger.info(f"Created PR #{pr.number} in {repo_name}")
            return pr
            
        except GithubException as e:
            logger.error(f"Failed to create PR: {str(e)}")
            raise
    
    async def review_pull_request(self, repo_name: str, pr_number: int) -> PRReview:
        """Review a pull request using AI."""
        try:
            repo = self.github.get_repo(repo_name)
            pr = repo.get_pull(pr_number)
            
            # Get PR details
            files = list(pr.get_files())
            
            review = PRReview(
                pr_number=pr_number,
                title=pr.title,
                description=pr.body or "",
                files_changed=[f.filename for f in files],
                additions=pr.additions,
                deletions=pr.deletions,
                suggestions=[],
                approval_status="pending",
                comments=[]
            )
            
            # Analyze each file
            for file in files:
                if file.patch:
                    analysis = await self._analyze_code_changes(file.filename, file.patch)
                    if analysis['suggestions']:
                        review.suggestions.extend(analysis['suggestions'])
                    if analysis['issues']:
                        review.comments.extend(analysis['issues'])
            
            # Determine approval status
            if not review.comments:
                review.approval_status = "approved"
            elif any('critical' in c.lower() for c in review.comments):
                review.approval_status = "changes_requested"
            else:
                review.approval_status = "commented"
            
            # Post review
            await self._post_pr_review(repo, pr, review)
            
            return review
            
        except Exception as e:
            logger.error(f"Failed to review PR: {str(e)}")
            raise
    
    async def monitor_repository(
        self,
        repo_name: str,
        callbacks: Dict[str, Callable],
        interval: int = 60
    ):
        """Monitor a repository for changes."""
        async def _monitor():
            repo = self.github.get_repo(repo_name)
            last_state = {
                'commit': repo.get_commits()[0].sha if repo.get_commits().totalCount > 0 else None,
                'issues': [i.number for i in repo.get_issues(state='open')],
                'prs': [pr.number for pr in repo.get_pulls(state='open')]
            }
            
            while True:
                try:
                    await asyncio.sleep(interval)
                    
                    # Check for new commits
                    current_commit = repo.get_commits()[0].sha if repo.get_commits().totalCount > 0 else None
                    if current_commit != last_state['commit'] and 'on_commit' in callbacks:
                        await callbacks['on_commit'](repo, current_commit)
                        last_state['commit'] = current_commit
                    
                    # Check for new issues
                    current_issues = [i.number for i in repo.get_issues(state='open')]
                    new_issues = set(current_issues) - set(last_state['issues'])
                    if new_issues and 'on_issue' in callbacks:
                        for issue_num in new_issues:
                            issue = repo.get_issue(issue_num)
                            await callbacks['on_issue'](repo, issue)
                        last_state['issues'] = current_issues
                    
                    # Check for new PRs
                    current_prs = [pr.number for pr in repo.get_pulls(state='open')]
                    new_prs = set(current_prs) - set(last_state['prs'])
                    if new_prs and 'on_pr' in callbacks:
                        for pr_num in new_prs:
                            pr = repo.get_pull(pr_num)
                            await callbacks['on_pr'](repo, pr)
                        last_state['prs'] = current_prs
                        
                except Exception as e:
                    logger.error(f"Error monitoring repository {repo_name}: {str(e)}")
        
        # Start monitoring task
        task = asyncio.create_task(_monitor())
        self.monitors[repo_name] = task
        
    async def stop_monitoring(self, repo_name: str):
        """Stop monitoring a repository."""
        if repo_name in self.monitors:
            self.monitors[repo_name].cancel()
            del self.monitors[repo_name]
    
    async def execute_github_action(self, action: Action) -> OperationResult:
        """Execute a GitHub-specific action."""
        try:
            github_action = action.metadata.get('github_action')
            
            if github_action == 'create_issue':
                issue = await self._create_issue(action.metadata)
                return OperationResult(
                    operation_id=action.id,
                    status=OperationStatus.SUCCESS,
                    result={'issue_number': issue.number, 'url': issue.html_url}
                )
                
            elif github_action == 'merge_pr':
                pr = await self._merge_pr(action.metadata)
                return OperationResult(
                    operation_id=action.id,
                    status=OperationStatus.SUCCESS,
                    result={'merged': True, 'sha': pr.merge_commit_sha}
                )
                
            elif github_action == 'run_workflow':
                run = await self._trigger_workflow(action.metadata)
                return OperationResult(
                    operation_id=action.id,
                    status=OperationStatus.SUCCESS,
                    result={'run_id': run['id'], 'url': run['html_url']}
                )
                
            else:
                raise ValueError(f"Unknown GitHub action: {github_action}")
                
        except Exception as e:
            return OperationResult(
                operation_id=action.id,
                status=OperationStatus.FAILED,
                error=e
            )
    
    async def _parse_npm_dependencies(self, package_path: Path) -> List[str]:
        """Parse NPM dependencies."""
        async with aiofiles.open(package_path, 'r') as f:
            content = await f.read()
            data = json.loads(content)
            deps = []
            for dep_type in ['dependencies', 'devDependencies']:
                if dep_type in data:
                    deps.extend(data[dep_type].keys())
            return deps
    
    async def _parse_python_requirements(self, req_path: Path) -> List[str]:
        """Parse Python requirements."""
        deps = []
        async with aiofiles.open(req_path, 'r') as f:
            async for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    # Extract package name
                    pkg = line.split('==')[0].split('>=')[0].split('<=')[0]
                    deps.append(pkg)
        return deps
    
    async def _parse_cargo_dependencies(self, cargo_path: Path) -> List[str]:
        """Parse Cargo dependencies."""
        # Simplified parsing - in production use toml library
        deps = []
        in_deps = False
        async with aiofiles.open(cargo_path, 'r') as f:
            async for line in f:
                if '[dependencies]' in line:
                    in_deps = True
                elif line.startswith('[') and in_deps:
                    break
                elif in_deps and '=' in line:
                    dep = line.split('=')[0].strip()
                    deps.append(dep)
        return deps
    
    async def _parse_go_dependencies(self, go_mod_path: Path) -> List[str]:
        """Parse Go dependencies."""
        deps = []
        async with aiofiles.open(go_mod_path, 'r') as f:
            async for line in f:
                if line.strip().startswith('require'):
                    parts = line.strip().split()
                    if len(parts) >= 2:
                        deps.append(parts[1])
        return deps
    
    async def _parse_maven_dependencies(self, pom_path: Path) -> List[str]:
        """Parse Maven dependencies."""
        # Simplified - in production use XML parser
        deps = []
        async with aiofiles.open(pom_path, 'r') as f:
            content = await f.read()
            # Extract artifactIds
            import re
            artifacts = re.findall(r'<artifactId>(.*?)</artifactId>', content)
            deps.extend(artifacts)
        return list(set(deps))
    
    async def _analyze_structure(self, repo_path: str) -> Dict[str, Any]:
        """Analyze repository structure."""
        structure = {
            'directories': {},
            'total_files': 0,
            'total_lines': 0
        }
        
        for root, dirs, files in os.walk(repo_path):
            rel_root = os.path.relpath(root, repo_path)
            if rel_root == '.':
                rel_root = ''
            
            # Count files per directory
            structure['directories'][rel_root] = len(files)
            structure['total_files'] += len(files)
            
        return structure
    
    async def _analyze_code_changes(self, filename: str, patch: str) -> Dict[str, List[str]]:
        """Analyze code changes using AI."""
        if not self.ai_model:
            return {'suggestions': [], 'issues': []}
        
        # Use AI to analyze the patch
        prompt = f"""
        Analyze this code change in {filename}:
        
        {patch}
        
        Provide:
        1. Potential issues or bugs
        2. Suggestions for improvement
        3. Security concerns
        """
        
        # This would use the AI model to analyze
        # For now, return basic analysis
        analysis = {'suggestions': [], 'issues': []}
        
        # Basic checks
        if 'password' in patch.lower() or 'secret' in patch.lower():
            analysis['issues'].append("Potential sensitive data exposure")
        
        if 'TODO' in patch or 'FIXME' in patch:
            analysis['suggestions'].append("Contains TODO/FIXME comments that should be addressed")
        
        return analysis
    
    async def _post_pr_review(self, repo: Repository, pr: PullRequest, review: PRReview):
        """Post review comments to PR."""
        if review.comments:
            comment_body = "## Automated Review Results\n\n"
            comment_body += f"**Status:** {review.approval_status}\n\n"
            
            if review.comments:
                comment_body += "### Issues Found:\n"
                for comment in review.comments:
                    comment_body += f"- {comment}\n"
            
            if review.suggestions:
                comment_body += "\n### Suggestions:\n"
                for suggestion in review.suggestions:
                    comment_body += f"- {suggestion}\n"
            
            pr.create_issue_comment(comment_body)
    
    async def _create_issue(self, metadata: Dict[str, Any]) -> Issue:
        """Create a GitHub issue."""
        repo = self.github.get_repo(metadata['repo_name'])
        issue = repo.create_issue(
            title=metadata['title'],
            body=metadata.get('body', ''),
            labels=metadata.get('labels', [])
        )
        return issue
    
    async def _merge_pr(self, metadata: Dict[str, Any]) -> PullRequest:
        """Merge a pull request."""
        repo = self.github.get_repo(metadata['repo_name'])
        pr = repo.get_pull(metadata['pr_number'])
        pr.merge(
            commit_title=metadata.get('commit_title', f"Merge PR #{pr.number}"),
            commit_message=metadata.get('commit_message', ''),
            merge_method=metadata.get('merge_method', 'merge')
        )
        return pr
    
    async def _trigger_workflow(self, metadata: Dict[str, Any]) -> Dict[str, Any]:
        """Trigger a GitHub Actions workflow."""
        repo = self.github.get_repo(metadata['repo_name'])
        workflow = repo.get_workflow(metadata['workflow_id'])
        
        # Trigger the workflow
        workflow.create_dispatch(
            ref=metadata.get('ref', 'main'),
            inputs=metadata.get('inputs', {})
        )
        
        # Get the run details
        runs = list(workflow.get_runs(branch=metadata.get('ref', 'main')))[0]
        return {
            'id': runs.id,
            'html_url': runs.html_url,
            'status': runs.status
        }
    
    async def cleanup(self):
        """Cleanup resources."""
        # Cancel all monitoring tasks
        for task in self.monitors.values():
            task.cancel()
        
        # Close HTTP client
        await self._client.aclose()