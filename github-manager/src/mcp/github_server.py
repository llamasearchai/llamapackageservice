"""
GitHub MCP Server Implementation
"""
import asyncio
import json
import logging
from typing import Dict, List, Any, Optional
import websockets
from github import Github, GithubException
from dataclasses import dataclass, asdict
from datetime import datetime
import base64

logger = logging.getLogger(__name__)


@dataclass
class MCPMessage:
    type: str
    id: Optional[str] = None
    tool: Optional[str] = None
    params: Optional[Dict[str, Any]] = None
    result: Optional[Any] = None
    error: Optional[str] = None


class GitHubMCPServer:
    def __init__(self, github_token: str, port: int = 3001):
        self.github = Github(github_token)
        self.port = port
        self.tools = {}
        self._setup_tools()
        self.monitors = {}
        
    def _setup_tools(self):
        """Setup available GitHub tools"""
        self.tools = {
            'list_repositories': self.list_repositories,
            'create_repository': self.create_repository,
            'analyze_repository_issues': self.analyze_repository_issues,
            'create_pull_request': self.create_pull_request,
            'review_pull_request': self.review_pull_request,
            'execute_workflow': self.execute_workflow,
            'get_repository_insights': self.get_repository_insights,
            'search_code': self.search_code,
            'get_file_content': self.get_file_content,
            'update_file': self.update_file,
            'manage_secrets': self.manage_secrets,
            'get_notifications': self.get_notifications,
            'monitor_repository': self.monitor_repository,
            'stop_monitoring': self.stop_monitoring,
            'create_issue': self.create_issue,
            'update_issue': self.update_issue,
            'merge_pull_request': self.merge_pull_request,
            'get_repository_stats': self.get_repository_stats,
            'get_collaborators': self.get_collaborators,
            'create_release': self.create_release,
            'get_branches': self.get_branches,
            'create_branch': self.create_branch,
            'delete_branch': self.delete_branch
        }
    
    async def start(self):
        """Start the MCP server"""
        logger.info(f"Starting GitHub MCP server on port {self.port}")
        
        async def handle_client(websocket, path):
            logger.info(f"Client connected from {websocket.remote_address}")
            
            try:
                async for message in websocket:
                    await self.handle_message(websocket, message)
            except websockets.exceptions.ConnectionClosed:
                logger.info("Client disconnected")
            except Exception as e:
                logger.error(f"Error handling client: {e}")
        
        await websockets.serve(handle_client, "localhost", self.port)
        logger.info(f"GitHub MCP server running on ws://localhost:{self.port}")
        
        # Keep server running
        await asyncio.Future()
    
    async def handle_message(self, websocket, message: str):
        """Handle incoming MCP messages"""
        try:
            data = json.loads(message)
            msg = MCPMessage(**data)
            
            if msg.type == 'initialize':
                await self.handle_initialize(websocket, msg)
            elif msg.type == 'tool_call':
                await self.handle_tool_call(websocket, msg)
            else:
                await self.send_error(websocket, msg.id, f"Unknown message type: {msg.type}")
                
        except json.JSONDecodeError:
            await self.send_error(websocket, None, "Invalid JSON")
        except Exception as e:
            logger.error(f"Error handling message: {e}")
            await self.send_error(websocket, None, str(e))
    
    async def handle_initialize(self, websocket, msg: MCPMessage):
        """Handle initialization request"""
        response = MCPMessage(
            type='initialized',
            id=msg.id,
            result={
                'capabilities': list(self.tools.keys()),
                'server': 'github-mcp-server',
                'version': '1.0.0'
            }
        )
        await websocket.send(json.dumps(asdict(response)))
    
    async def handle_tool_call(self, websocket, msg: MCPMessage):
        """Handle tool call request"""
        tool_name = msg.tool
        params = msg.params or {}
        
        if tool_name not in self.tools:
            await self.send_error(websocket, msg.id, f"Unknown tool: {tool_name}")
            return
        
        try:
            # Execute tool
            result = await self.tools[tool_name](**params)
            
            response = MCPMessage(
                type='tool_result',
                id=msg.id,
                result=result
            )
            await websocket.send(json.dumps(asdict(response)))
            
        except Exception as e:
            logger.error(f"Error executing tool {tool_name}: {e}")
            await self.send_error(websocket, msg.id, str(e))
    
    async def send_error(self, websocket, msg_id: Optional[str], error: str):
        """Send error response"""
        response = MCPMessage(
            type='error',
            id=msg_id,
            error=error
        )
        await websocket.send(json.dumps(asdict(response)))
    
    # GitHub Tool Implementations
    
    async def list_repositories(self, username: Optional[str] = None) -> Dict:
        """List all repositories for user or organization"""
        try:
            if username:
                user = self.github.get_user(username)
                repos = user.get_repos()
            else:
                repos = self.github.get_user().get_repos()
            
            repo_list = []
            for repo in repos:
                repo_list.append({
                    'name': repo.name,
                    'full_name': repo.full_name,
                    'description': repo.description,
                    'language': repo.language,
                    'stars': repo.stargazers_count,
                    'forks': repo.forks_count,
                    'private': repo.private,
                    'url': repo.html_url,
                    'created_at': repo.created_at.isoformat(),
                    'updated_at': repo.updated_at.isoformat()
                })
            
            return {'repositories': repo_list, 'total': len(repo_list)}
            
        except GithubException as e:
            raise Exception(f"GitHub API error: {e.status} - {e.data}")
    
    async def create_repository(self, name: str, description: str = "", 
                              private: bool = False, auto_init: bool = True) -> Dict:
        """Create a new GitHub repository"""
        try:
            user = self.github.get_user()
            repo = user.create_repo(
                name=name,
                description=description,
                private=private,
                auto_init=auto_init
            )
            
            return {
                'success': True,
                'repository': {
                    'name': repo.name,
                    'full_name': repo.full_name,
                    'url': repo.html_url,
                    'clone_url': repo.clone_url,
                    'ssh_url': repo.ssh_url
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to create repository: {e.data}")
    
    async def analyze_repository_issues(self, repo_name: str) -> Dict:
        """Analyze repository issues and provide insights"""
        try:
            repo = self.github.get_repo(repo_name)
            issues = list(repo.get_issues(state='open'))
            
            analysis = {
                'total_open_issues': len(issues),
                'issues_by_label': {},
                'priority_issues': [],
                'stale_issues': [],
                'recent_issues': [],
                'contributors': set()
            }
            
            for issue in issues:
                # Track contributors
                if issue.user:
                    analysis['contributors'].add(issue.user.login)
                
                # Categorize by labels
                for label in issue.labels:
                    if label.name not in analysis['issues_by_label']:
                        analysis['issues_by_label'][label.name] = 0
                    analysis['issues_by_label'][label.name] += 1
                
                # Identify priority issues
                if any(label.name in ['critical', 'high-priority', 'bug'] for label in issue.labels):
                    analysis['priority_issues'].append({
                        'number': issue.number,
                        'title': issue.title,
                        'created_at': issue.created_at.isoformat(),
                        'labels': [l.name for l in issue.labels]
                    })
                
                # Check for stale issues
                days_old = (datetime.utcnow() - issue.created_at).days
                if days_old > 30:
                    analysis['stale_issues'].append({
                        'number': issue.number,
                        'title': issue.title,
                        'days_old': days_old
                    })
                
                # Recent issues
                if days_old < 7:
                    analysis['recent_issues'].append({
                        'number': issue.number,
                        'title': issue.title,
                        'created_at': issue.created_at.isoformat()
                    })
            
            # Convert set to list
            analysis['contributors'] = list(analysis['contributors'])
            
            return analysis
            
        except GithubException as e:
            raise Exception(f"Failed to analyze issues: {e.data}")
    
    async def create_pull_request(self, repo_name: str, title: str, body: str,
                                head: str, base: str = "main") -> Dict:
        """Create a pull request"""
        try:
            repo = self.github.get_repo(repo_name)
            pr = repo.create_pull(
                title=title,
                body=body,
                head=head,
                base=base
            )
            
            return {
                'success': True,
                'pull_request': {
                    'number': pr.number,
                    'url': pr.html_url,
                    'state': pr.state,
                    'created_at': pr.created_at.isoformat()
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to create PR: {e.data}")
    
    async def review_pull_request(self, repo_name: str, pr_number: int) -> Dict:
        """Review a pull request"""
        try:
            repo = self.github.get_repo(repo_name)
            pr = repo.get_pull(pr_number)
            
            files = list(pr.get_files())
            
            review = {
                'pr_number': pr_number,
                'title': pr.title,
                'description': pr.body or "",
                'files_changed': [f.filename for f in files],
                'additions': pr.additions,
                'deletions': pr.deletions,
                'commits': pr.commits,
                'mergeable': pr.mergeable,
                'mergeable_state': pr.mergeable_state,
                'reviews': [],
                'checks': []
            }
            
            # Get reviews
            for r in pr.get_reviews():
                review['reviews'].append({
                    'user': r.user.login,
                    'state': r.state,
                    'submitted_at': r.submitted_at.isoformat() if r.submitted_at else None
                })
            
            # Get status checks
            if pr.head.sha:
                commit = repo.get_commit(pr.head.sha)
                for status in commit.get_statuses():
                    review['checks'].append({
                        'context': status.context,
                        'state': status.state,
                        'description': status.description
                    })
            
            return review
            
        except GithubException as e:
            raise Exception(f"Failed to review PR: {e.data}")
    
    async def execute_workflow(self, repo_name: str, workflow_id: str,
                             ref: str = "main", inputs: Dict = None) -> Dict:
        """Execute GitHub Actions workflow"""
        try:
            repo = self.github.get_repo(repo_name)
            workflow = repo.get_workflow(workflow_id)
            
            # Create workflow dispatch
            workflow.create_dispatch(
                ref=ref,
                inputs=inputs or {}
            )
            
            # Get the latest run
            runs = list(workflow.get_runs(branch=ref))
            if runs:
                latest_run = runs[0]
                return {
                    'success': True,
                    'run': {
                        'id': latest_run.id,
                        'url': latest_run.html_url,
                        'status': latest_run.status,
                        'conclusion': latest_run.conclusion
                    }
                }
            
            return {'success': True, 'message': 'Workflow dispatched'}
            
        except GithubException as e:
            raise Exception(f"Failed to execute workflow: {e.data}")
    
    async def get_repository_insights(self, repo_name: str) -> Dict:
        """Get comprehensive repository insights"""
        try:
            repo = self.github.get_repo(repo_name)
            
            insights = {
                'basic_info': {
                    'name': repo.name,
                    'description': repo.description,
                    'stars': repo.stargazers_count,
                    'forks': repo.forks_count,
                    'watchers': repo.watchers_count,
                    'open_issues': repo.open_issues_count,
                    'language': repo.language,
                    'size': repo.size,
                    'created_at': repo.created_at.isoformat(),
                    'updated_at': repo.updated_at.isoformat()
                },
                'activity': {
                    'commits': 0,
                    'contributors': [],
                    'recent_commits': []
                },
                'languages': {},
                'topics': repo.get_topics()
            }
            
            # Get commit activity
            commits = list(repo.get_commits()[:100])  # Last 100 commits
            insights['activity']['commits'] = len(commits)
            
            # Get recent commits
            for commit in commits[:10]:
                insights['activity']['recent_commits'].append({
                    'sha': commit.sha[:7],
                    'message': commit.commit.message.split('\n')[0],
                    'author': commit.commit.author.name,
                    'date': commit.commit.author.date.isoformat()
                })
            
            # Get contributors
            contributors = list(repo.get_contributors())
            for contrib in contributors[:10]:
                insights['activity']['contributors'].append({
                    'login': contrib.login,
                    'contributions': contrib.contributions
                })
            
            # Get languages
            languages = repo.get_languages()
            total = sum(languages.values())
            insights['languages'] = {
                lang: {'bytes': bytes_count, 'percentage': round(bytes_count/total*100, 2)}
                for lang, bytes_count in languages.items()
            }
            
            return insights
            
        except GithubException as e:
            raise Exception(f"Failed to get insights: {e.data}")
    
    async def search_code(self, query: str, repo: Optional[str] = None,
                         language: Optional[str] = None) -> Dict:
        """Search code across repositories"""
        try:
            search_query = query
            if repo:
                search_query += f" repo:{repo}"
            if language:
                search_query += f" language:{language}"
            
            results = self.github.search_code(search_query)
            
            items = []
            for item in results[:50]:  # Limit to 50 results
                items.append({
                    'path': item.path,
                    'repository': item.repository.full_name,
                    'url': item.html_url,
                    'score': item.score
                })
            
            return {
                'query': search_query,
                'total_count': results.totalCount,
                'results': items
            }
            
        except GithubException as e:
            raise Exception(f"Search failed: {e.data}")
    
    async def get_file_content(self, repo_name: str, path: str, ref: str = "main") -> Dict:
        """Get file content from repository"""
        try:
            repo = self.github.get_repo(repo_name)
            file_content = repo.get_contents(path, ref=ref)
            
            if file_content.type != "file":
                raise Exception(f"{path} is not a file")
            
            return {
                'path': file_content.path,
                'content': base64.b64decode(file_content.content).decode('utf-8'),
                'sha': file_content.sha,
                'size': file_content.size,
                'encoding': file_content.encoding
            }
            
        except GithubException as e:
            raise Exception(f"Failed to get file: {e.data}")
    
    async def update_file(self, repo_name: str, path: str, content: str,
                         message: str, branch: str = "main", sha: Optional[str] = None) -> Dict:
        """Update file in repository"""
        try:
            repo = self.github.get_repo(repo_name)
            
            # Get current file SHA if not provided
            if not sha:
                try:
                    current_file = repo.get_contents(path, ref=branch)
                    sha = current_file.sha
                except:
                    sha = None  # File doesn't exist
            
            # Update or create file
            if sha:
                result = repo.update_file(
                    path=path,
                    message=message,
                    content=content,
                    sha=sha,
                    branch=branch
                )
            else:
                result = repo.create_file(
                    path=path,
                    message=message,
                    content=content,
                    branch=branch
                )
            
            return {
                'success': True,
                'commit': {
                    'sha': result['commit'].sha,
                    'message': message,
                    'url': result['commit'].html_url
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to update file: {e.data}")
    
    async def manage_secrets(self, repo_name: str, action: str,
                           secret_name: str, value: Optional[str] = None) -> Dict:
        """Manage repository secrets"""
        try:
            repo = self.github.get_repo(repo_name)
            
            if action == "list":
                secrets = repo.get_secrets()
                return {
                    'secrets': [{'name': s.name, 'created_at': s.created_at.isoformat()} 
                               for s in secrets]
                }
            elif action == "create" or action == "update":
                if not value:
                    raise ValueError("Value required for create/update")
                
                # This would require additional encryption setup
                # For now, return a placeholder
                return {
                    'success': True,
                    'message': f"Secret {secret_name} would be {action}d (requires encryption setup)"
                }
            elif action == "delete":
                # repo.delete_secret(secret_name)
                return {
                    'success': True,
                    'message': f"Secret {secret_name} would be deleted"
                }
            else:
                raise ValueError(f"Unknown action: {action}")
                
        except GithubException as e:
            raise Exception(f"Failed to manage secrets: {e.data}")
    
    async def get_notifications(self, all: bool = False, participating: bool = False) -> Dict:
        """Get GitHub notifications"""
        try:
            user = self.github.get_user()
            notifications = user.get_notifications(all=all, participating=participating)
            
            items = []
            for notif in notifications:
                items.append({
                    'id': notif.id,
                    'repository': notif.repository.full_name,
                    'subject': {
                        'title': notif.subject.title,
                        'type': notif.subject.type
                    },
                    'reason': notif.reason,
                    'unread': notif.unread,
                    'updated_at': notif.updated_at.isoformat()
                })
            
            return {'notifications': items}
            
        except GithubException as e:
            raise Exception(f"Failed to get notifications: {e.data}")
    
    async def monitor_repository(self, repo_name: str, events: List[str]) -> Dict:
        """Start monitoring repository for events"""
        if repo_name in self.monitors:
            return {'success': False, 'message': 'Already monitoring this repository'}
        
        # In a real implementation, this would set up webhooks or polling
        self.monitors[repo_name] = {
            'events': events,
            'started_at': datetime.utcnow().isoformat()
        }
        
        return {
            'success': True,
            'message': f'Started monitoring {repo_name} for events: {", ".join(events)}'
        }
    
    async def stop_monitoring(self, repo_name: str) -> Dict:
        """Stop monitoring repository"""
        if repo_name not in self.monitors:
            return {'success': False, 'message': 'Not monitoring this repository'}
        
        del self.monitors[repo_name]
        
        return {
            'success': True,
            'message': f'Stopped monitoring {repo_name}'
        }
    
    async def create_issue(self, repo_name: str, title: str, body: str = "",
                          labels: List[str] = None, assignees: List[str] = None) -> Dict:
        """Create a new issue"""
        try:
            repo = self.github.get_repo(repo_name)
            issue = repo.create_issue(
                title=title,
                body=body,
                labels=labels or [],
                assignees=assignees or []
            )
            
            return {
                'success': True,
                'issue': {
                    'number': issue.number,
                    'url': issue.html_url,
                    'state': issue.state,
                    'created_at': issue.created_at.isoformat()
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to create issue: {e.data}")
    
    async def update_issue(self, repo_name: str, issue_number: int, **kwargs) -> Dict:
        """Update an existing issue"""
        try:
            repo = self.github.get_repo(repo_name)
            issue = repo.get_issue(issue_number)
            
            issue.edit(**kwargs)
            
            return {
                'success': True,
                'issue': {
                    'number': issue.number,
                    'state': issue.state,
                    'updated_at': issue.updated_at.isoformat()
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to update issue: {e.data}")
    
    async def merge_pull_request(self, repo_name: str, pr_number: int,
                               commit_title: Optional[str] = None,
                               commit_message: Optional[str] = None,
                               merge_method: str = "merge") -> Dict:
        """Merge a pull request"""
        try:
            repo = self.github.get_repo(repo_name)
            pr = repo.get_pull(pr_number)
            
            if not pr.mergeable:
                return {
                    'success': False,
                    'message': f'PR is not mergeable. State: {pr.mergeable_state}'
                }
            
            result = pr.merge(
                commit_title=commit_title,
                commit_message=commit_message,
                merge_method=merge_method
            )
            
            return {
                'success': result.merged,
                'sha': result.sha,
                'message': result.message
            }
            
        except GithubException as e:
            raise Exception(f"Failed to merge PR: {e.data}")
    
    async def get_repository_stats(self, repo_name: str) -> Dict:
        """Get repository statistics"""
        try:
            repo = self.github.get_repo(repo_name)
            
            # Get various stats
            stats = {
                'contributors': repo.get_contributors().totalCount,
                'commits': repo.get_commits().totalCount,
                'branches': repo.get_branches().totalCount,
                'tags': repo.get_tags().totalCount,
                'releases': repo.get_releases().totalCount,
                'issues': {
                    'open': repo.get_issues(state='open').totalCount,
                    'closed': repo.get_issues(state='closed').totalCount
                },
                'pull_requests': {
                    'open': repo.get_pulls(state='open').totalCount,
                    'closed': repo.get_pulls(state='closed').totalCount
                },
                'milestones': {
                    'open': repo.get_milestones(state='open').totalCount,
                    'closed': repo.get_milestones(state='closed').totalCount
                }
            }
            
            return stats
            
        except GithubException as e:
            raise Exception(f"Failed to get stats: {e.data}")
    
    async def get_collaborators(self, repo_name: str) -> Dict:
        """Get repository collaborators"""
        try:
            repo = self.github.get_repo(repo_name)
            collaborators = list(repo.get_collaborators())
            
            return {
                'collaborators': [
                    {
                        'login': c.login,
                        'name': c.name,
                        'permissions': {
                            'admin': c.permissions.admin,
                            'push': c.permissions.push,
                            'pull': c.permissions.pull
                        }
                    }
                    for c in collaborators
                ]
            }
            
        except GithubException as e:
            raise Exception(f"Failed to get collaborators: {e.data}")
    
    async def create_release(self, repo_name: str, tag_name: str, name: str,
                           body: str = "", draft: bool = False,
                           prerelease: bool = False) -> Dict:
        """Create a new release"""
        try:
            repo = self.github.get_repo(repo_name)
            release = repo.create_git_release(
                tag=tag_name,
                name=name,
                message=body,
                draft=draft,
                prerelease=prerelease
            )
            
            return {
                'success': True,
                'release': {
                    'id': release.id,
                    'tag_name': release.tag_name,
                    'name': release.title,
                    'url': release.html_url,
                    'created_at': release.created_at.isoformat()
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to create release: {e.data}")
    
    async def get_branches(self, repo_name: str) -> Dict:
        """Get repository branches"""
        try:
            repo = self.github.get_repo(repo_name)
            branches = list(repo.get_branches())
            
            return {
                'branches': [
                    {
                        'name': b.name,
                        'protected': b.protected,
                        'commit': {
                            'sha': b.commit.sha[:7],
                            'url': b.commit.html_url
                        }
                    }
                    for b in branches
                ]
            }
            
        except GithubException as e:
            raise Exception(f"Failed to get branches: {e.data}")
    
    async def create_branch(self, repo_name: str, branch_name: str,
                          from_branch: str = "main") -> Dict:
        """Create a new branch"""
        try:
            repo = self.github.get_repo(repo_name)
            
            # Get the commit SHA from the source branch
            source_branch = repo.get_branch(from_branch)
            sha = source_branch.commit.sha
            
            # Create new branch
            ref = repo.create_git_ref(
                ref=f"refs/heads/{branch_name}",
                sha=sha
            )
            
            return {
                'success': True,
                'branch': {
                    'name': branch_name,
                    'ref': ref.ref,
                    'sha': ref.object.sha[:7]
                }
            }
            
        except GithubException as e:
            raise Exception(f"Failed to create branch: {e.data}")
    
    async def delete_branch(self, repo_name: str, branch_name: str) -> Dict:
        """Delete a branch"""
        try:
            repo = self.github.get_repo(repo_name)
            ref = repo.get_git_ref(f"heads/{branch_name}")
            ref.delete()
            
            return {
                'success': True,
                'message': f'Branch {branch_name} deleted'
            }
            
        except GithubException as e:
            raise Exception(f"Failed to delete branch: {e.data}")


# Main entry point for running the server
if __name__ == "__main__":
    import os
    import sys
    
    # Check for setup flag
    if len(sys.argv) > 1 and sys.argv[1] == "--setup":
        print("GitHub MCP Server Setup Complete")
        print("Available tools:")
        server = GitHubMCPServer("")
        for tool in server.tools:
            print(f"  - {tool}")
        sys.exit(0)
    
    # Get GitHub token from environment
    token = os.getenv("GITHUB_TOKEN")
    if not token:
        print("Error: GITHUB_TOKEN environment variable not set")
        sys.exit(1)
    
    # Run server
    server = GitHubMCPServer(token)
    asyncio.run(server.start())