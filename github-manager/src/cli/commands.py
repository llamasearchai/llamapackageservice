"""
CLI Commands for GitHub Repository Manager
"""
import click
import asyncio
import json
from pathlib import Path
from typing import Optional
from tabulate import tabulate
import yaml

from ..core.repo_manager import GitHubRepoManager
from ..utils.logger import setup_logger

logger = setup_logger(__name__)


@click.group()
@click.option('--config', default='config/github_config.yaml', help='Configuration file path')
@click.pass_context
def cli(ctx, config):
    """GitHub Repository Manager with AI capabilities"""
    ctx.ensure_object(dict)
    ctx.obj['config'] = config


@cli.command()
@click.pass_context
def list_repos(ctx):
    """List all local repositories"""
    async def _list():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        repos = manager.scan_local_repositories()
        
        if not repos:
            click.echo("No repositories found")
            return
        
        # Format for display
        table_data = []
        for repo in repos:
            status = "Clean" if not repo['status'].get('is_dirty') else "Modified"
            table_data.append([
                repo['name'],
                repo['current_branch'],
                status,
                repo['status'].get('modified_files', 0),
                repo['last_commit']['sha']
            ])
        
        headers = ['Repository', 'Branch', 'Status', 'Modified', 'Last Commit']
        click.echo(tabulate(table_data, headers=headers, tablefmt='grid'))
        
        await manager.cleanup()
    
    asyncio.run(_list())


@cli.command()
@click.argument('repo-path')
@click.option('--detailed', is_flag=True, help='Show detailed analysis')
@click.pass_context
def analyze(ctx, repo_path, detailed):
    """Analyze repository using AI"""
    async def _analyze():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo(f"Analyzing repository: {repo_path}")
        
        with click.progressbar(length=100, label='Analyzing') as bar:
            bar.update(20)
            result = await manager.ai_analyze_repository(repo_path)
            bar.update(80)
        
        if detailed:
            click.echo(json.dumps(result, indent=2))
        else:
            click.echo("\n=== Repository Analysis ===")
            if 'analysis' in result:
                click.echo(result['analysis'])
            if 'visual_analyses' in result:
                click.echo("\n=== Visual Analysis ===")
                for va in result['visual_analyses']:
                    click.echo(f"\n{va['file']}:")
                    click.echo(va['analysis'])
        
        await manager.cleanup()
    
    asyncio.run(_analyze())


@cli.command()
@click.argument('command')
@click.option('--repo', help='Repository to operate on')
@click.pass_context
def ai_command(ctx, command, repo):
    """Execute natural language commands using AI"""
    async def _execute():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        # Use Ollama to interpret and execute command
        prompt = f"Command: {command}"
        if repo:
            prompt += f"\nRepository: {repo}"
        
        result = await manager.ollama.analyze_code_with_tools(
            repo or ".", 
            prompt
        )
        
        click.echo(f"\n{result['analysis']}")
        
        if result['tool_calls_made'] > 0:
            click.echo(f"\nTools used: {', '.join(result['tools_used'])}")
        
        await manager.cleanup()
    
    asyncio.run(_execute())


@cli.command()
@click.argument('name')
@click.option('--description', default='', help='Repository description')
@click.option('--private', is_flag=True, help='Create private repository')
@click.pass_context
def create_repo(ctx, name, description, private):
    """Create a new GitHub repository"""
    async def _create():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo(f"Creating repository: {name}")
        
        result = await manager.create_repository(name, description, private)
        
        if result['success']:
            click.echo(f"✓ Repository created: {result['url']}")
            click.echo(f"✓ Cloned to: {result['local_path']}")
        else:
            click.echo(f"✗ Failed: {result['error']}")
        
        await manager.cleanup()
    
    asyncio.run(_create())


@cli.command()
@click.argument('repo-name')
@click.pass_context
def sync(ctx, repo_name):
    """Sync repository with remote"""
    async def _sync():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        result = await manager.sync_repository(repo_name)
        
        if result['success']:
            click.echo(f"✓ Repository synced: {result.get('action', 'updated')}")
        else:
            click.echo(f"✗ Failed: {result['error']}")
            if 'modified_files' in result:
                click.echo("Modified files:")
                for f in result['modified_files']:
                    click.echo(f"  - {f}")
        
        await manager.cleanup()
    
    asyncio.run(_sync())


@cli.command()
@click.argument('repo-name')
@click.pass_context
def issues(ctx, repo_name):
    """Analyze repository issues"""
    async def _issues():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo(f"Analyzing issues for: {repo_name}")
        
        result = await manager.analyze_issues(repo_name)
        
        if 'error' in result:
            click.echo(f"✗ Error: {result['error']}")
            return
        
        click.echo(f"\nTotal open issues: {result['total_open']}")
        
        if result['by_label']:
            click.echo("\nIssues by label:")
            for label, count in result['by_label'].items():
                click.echo(f"  {label}: {count}")
        
        if result['priority_issues']:
            click.echo("\nPriority issues:")
            for issue in result['priority_issues'][:5]:
                click.echo(f"  #{issue['number']}: {issue['title']}")
        
        if result['stale_issues']:
            click.echo(f"\nStale issues (>30 days): {len(result['stale_issues'])}")
        
        if 'ai_insights' in result:
            click.echo("\nAI Insights:")
            click.echo(result['ai_insights'])
        
        await manager.cleanup()
    
    asyncio.run(_issues())


@cli.command()
@click.argument('operation', type=click.Choice(['sync', 'analyze', 'backup', 'update_deps']))
@click.argument('repo-names', nargs=-1, required=True)
@click.pass_context
def batch(ctx, operation, repo_names):
    """Perform batch operations on multiple repositories"""
    async def _batch():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo(f"Performing {operation} on {len(repo_names)} repositories")
        
        with click.progressbar(repo_names) as repos:
            results = await manager.batch_operation(operation, list(repos))
        
        # Display results
        success_count = sum(1 for r in results.values() if r.get('success', False))
        click.echo(f"\nCompleted: {success_count}/{len(repo_names)} successful")
        
        # Show failures
        failures = {k: v for k, v in results.items() if not v.get('success', False)}
        if failures:
            click.echo("\nFailures:")
            for repo, result in failures.items():
                click.echo(f"  {repo}: {result.get('error', 'Unknown error')}")
        
        await manager.cleanup()
    
    asyncio.run(_batch())


@cli.command()
@click.option('--query', prompt='Search query', help='Natural language search query')
@click.option('--repo', help='Limit search to specific repository')
@click.option('--language', help='Filter by programming language')
@click.pass_context
def search(ctx, query, repo, language):
    """Search code across repositories"""
    async def _search():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo(f"Searching for: {query}")
        
        results = await manager.mcp_client.search_code(query, repo, language)
        
        if not results:
            click.echo("No results found")
            return
        
        click.echo(f"\nFound {len(results)} results:")
        for i, result in enumerate(results[:10], 1):
            click.echo(f"\n{i}. {result.get('path', 'Unknown')}")
            click.echo(f"   Repository: {result.get('repository', 'Unknown')}")
            if 'snippet' in result:
                click.echo(f"   {result['snippet']}")
        
        await manager.cleanup()
    
    asyncio.run(_search())


@cli.command()
@click.pass_context
def dashboard(ctx):
    """Start web dashboard"""
    click.echo("Starting web dashboard...")
    
    from ..web.app import create_app
    import uvicorn
    
    app = create_app(ctx.obj['config'])
    uvicorn.run(app, host="0.0.0.0", port=8000)


@cli.command()
@click.pass_context
def interactive(ctx):
    """Start interactive mode"""
    async def _interactive():
        manager = GitHubRepoManager(ctx.obj['config'])
        await manager.initialize()
        
        click.echo("GitHub Repository Manager - Interactive Mode")
        click.echo("Type 'help' for available commands or 'exit' to quit\n")
        
        while True:
            try:
                command = click.prompt('> ', default='', show_default=False)
                
                if command.lower() in ['exit', 'quit']:
                    break
                elif command.lower() == 'help':
                    click.echo("""
Available commands:
  list              - List all repositories
  analyze <repo>    - Analyze a repository
  sync <repo>       - Sync repository with remote
  create <name>     - Create new repository
  issues <repo>     - Analyze repository issues
  search <query>    - Search code
  ai <command>      - Execute natural language command
  exit              - Exit interactive mode
                    """)
                elif command.startswith('ai '):
                    ai_cmd = command[3:]
                    result = await manager.ollama.analyze_code_with_tools(".", ai_cmd)
                    click.echo(result['analysis'])
                else:
                    # Parse and execute command
                    parts = command.split()
                    if len(parts) == 0:
                        continue
                    
                    cmd = parts[0]
                    args = parts[1:] if len(parts) > 1 else []
                    
                    if cmd == 'list':
                        repos = manager.scan_local_repositories()
                        for repo in repos:
                            click.echo(f"- {repo['name']} ({repo['current_branch']})")
                    elif cmd == 'analyze' and args:
                        result = await manager.ai_analyze_repository(args[0])
                        click.echo(result.get('analysis', 'Analysis complete'))
                    elif cmd == 'sync' and args:
                        result = await manager.sync_repository(args[0])
                        if result['success']:
                            click.echo("Repository synced")
                        else:
                            click.echo(f"Sync failed: {result['error']}")
                    else:
                        click.echo(f"Unknown command: {cmd}")
                        
            except KeyboardInterrupt:
                break
            except Exception as e:
                click.echo(f"Error: {e}")
        
        await manager.cleanup()
        click.echo("\nGoodbye!")
    
    asyncio.run(_interactive())


if __name__ == '__main__':
    cli()