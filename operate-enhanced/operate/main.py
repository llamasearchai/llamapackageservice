"""Enhanced main entry point for the operate framework."""
import asyncio
import click
import logging
import sys
from pathlib import Path
from typing import Optional

from rich.console import Console
from rich.logging import RichHandler
from rich.progress import Progress, SpinnerColumn, TextColumn

from .interfaces import Action, ActionType
from .core.orchestrator import OperationOrchestrator, Operation
from .integrations.github_manager import GitHubManager
from .security.guardian import SecurityGuardian, SecurityConfig
from .state.manager import StateManager, StorageBackend
from .core.performance import PerformanceOptimizer
from .utils.action_handler import ActionHandler
from .models.model_manager import ModelManager
from .api.server import run_server


console = Console()

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(message)s",
    handlers=[RichHandler(console=console, rich_tracebacks=True)]
)
logger = logging.getLogger(__name__)


class OperateApp:
    """Main application class."""
    
    def __init__(
        self,
        model: str = "gpt-4o",
        sandbox: bool = True,
        github_token: Optional[str] = None,
        cache_enabled: bool = True
    ):
        self.model_name = model
        self.sandbox = sandbox
        self.github_token = github_token
        self.cache_enabled = cache_enabled
        
        # Initialize components
        self.security_guardian = SecurityGuardian(
            SecurityConfig(enable_sandbox=sandbox)
        )
        self.state_manager = StateManager(
            StorageBackend(type="file", path="./state")
        )
        self.performance_optimizer = PerformanceOptimizer() if cache_enabled else None
        
        # Initialize handlers
        self.action_handler = ActionHandler(performance_optimizer=self.performance_optimizer)
        self.model_manager = ModelManager(model_name=model)
        
        # Initialize orchestrator
        self.orchestrator = OperationOrchestrator(
            action_interface=self.action_handler,
            state_manager=self.state_manager,
            security_validator=self.security_guardian,
            enable_learning=True
        )
        
        # Initialize GitHub manager if token provided
        self.github_manager = None
        if github_token:
            self.github_manager = GitHubManager(github_token, ai_model=self.model_manager)
            
    async def initialize(self):
        """Initialize the application."""
        console.print("[bold green]Initializing Operate Enhanced...[/bold green]")
        
        if self.github_manager:
            await self.github_manager.initialize()
            console.print("✓ GitHub integration initialized")
            
        if self.sandbox:
            console.print("[yellow]⚠ Running in SANDBOX mode - actions are restricted[/yellow]")
        else:
            console.print("[red]⚠ Running in PRODUCTION mode - be careful![/red]")
            
    async def run_interactive(self, objective: str):
        """Run in interactive mode."""
        console.print(f"\n[bold]Objective:[/bold] {objective}\n")
        
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            console=console
        ) as progress:
            
            task = progress.add_task("Processing...", total=None)
            
            iteration = 0
            max_iterations = 10
            
            while iteration < max_iterations:
                iteration += 1
                progress.update(task, description=f"Iteration {iteration}/{max_iterations}")
                
                try:
                    # Capture screenshot
                    screenshot = await self.action_handler.capture_screenshot()
                    
                    # Get AI decision
                    if self.performance_optimizer:
                        action = await self.performance_optimizer.get_model_response(
                            screenshot,
                            objective,
                            lambda s, o: self.model_manager.analyze_screen(s, o)
                        )
                    else:
                        action = await self.model_manager.analyze_screen(screenshot, objective)
                    
                    # Check if done
                    if action.type == ActionType.WAIT and action.value == "done":
                        console.print("\n[bold green]✓ Objective completed![/bold green]")
                        break
                    
                    # Create and execute operation
                    operation = Operation(
                        action=action,
                        metadata={"iteration": iteration, "objective": objective}
                    )
                    
                    result = await self.orchestrator.execute_operation(operation)
                    
                    # Display result
                    if result.status.value == "success":
                        console.print(f"✓ {action.type.value}: {action.value or action.target}")
                    else:
                        console.print(f"✗ Failed: {result.error}")
                        
                except Exception as e:
                    console.print(f"[red]Error: {str(e)}[/red]")
                    break
                    
        # Show performance metrics
        if self.performance_optimizer:
            metrics = await self.performance_optimizer.get_metrics()
            console.print("\n[bold]Performance Metrics:[/bold]")
            console.print(f"Cache hit rate: {metrics['cache']['hit_rate']:.2%}")
            console.print(f"Avg operation time: {metrics['operations']['average_duration_ms']:.2f}ms")
            
    async def run_pattern(self, pattern_name: str):
        """Run a saved pattern."""
        actions = await self.state_manager.load_pattern(pattern_name)
        if not actions:
            console.print(f"[red]Pattern '{pattern_name}' not found[/red]")
            return
            
        console.print(f"[bold]Running pattern:[/bold] {pattern_name} ({len(actions)} actions)")
        
        operations = [Operation(action=action) for action in actions]
        results = await self.orchestrator.execute_sequence(operations)
        
        success_count = sum(1 for r in results if r.status.value == "success")
        console.print(f"\n✓ Completed {success_count}/{len(results)} actions successfully")
        
    async def cleanup(self):
        """Cleanup resources."""
        if self.github_manager:
            await self.github_manager.cleanup()
            
        # Save metrics
        if self.performance_optimizer:
            metrics = await self.performance_optimizer.get_metrics()
            await self.state_manager.save_state("last_session_metrics", metrics)


@click.group()
@click.version_option(version="2.0.0")
def cli():
    """Operate Enhanced - AI-powered computer automation with GitHub integration."""
    pass


@cli.command()
@click.argument('objective')
@click.option('--model', '-m', default='gpt-4o', help='AI model to use')
@click.option('--no-sandbox', is_flag=True, help='Disable sandbox mode (dangerous!)')
@click.option('--github-token', envvar='GITHUB_TOKEN', help='GitHub access token')
@click.option('--no-cache', is_flag=True, help='Disable performance caching')
def run(objective: str, model: str, no_sandbox: bool, github_token: str, no_cache: bool):
    """Run the operate framework with an objective."""
    async def _run():
        app = OperateApp(
            model=model,
            sandbox=not no_sandbox,
            github_token=github_token,
            cache_enabled=not no_cache
        )
        
        try:
            await app.initialize()
            await app.run_interactive(objective)
        finally:
            await app.cleanup()
            
    asyncio.run(_run())


@cli.command()
@click.argument('pattern_name')
@click.option('--no-sandbox', is_flag=True, help='Disable sandbox mode')
def pattern(pattern_name: str, no_sandbox: bool):
    """Run a saved pattern."""
    async def _run():
        app = OperateApp(sandbox=not no_sandbox)
        
        try:
            await app.initialize()
            await app.run_pattern(pattern_name)
        finally:
            await app.cleanup()
            
    asyncio.run(_run())


@cli.command()
@click.option('--host', default='0.0.0.0', help='Server host')
@click.option('--port', default=8000, help='Server port')
def server(host: str, port: int):
    """Start the web dashboard and API server."""
    console.print(f"[bold green]Starting server on {host}:{port}[/bold green]")
    console.print(f"Dashboard: http://localhost:{port}")
    console.print(f"API docs: http://localhost:{port}/docs")
    
    run_server(host, port)


@cli.command()
@click.argument('repo_name')
@click.option('--token', envvar='GITHUB_TOKEN', required=True, help='GitHub token')
@click.option('--pr-number', type=int, help='PR number to review')
def github(repo_name: str, token: str, pr_number: int):
    """GitHub repository operations."""
    async def _run():
        manager = GitHubManager(token)
        await manager.initialize()
        
        if pr_number:
            console.print(f"[bold]Reviewing PR #{pr_number} in {repo_name}[/bold]")
            review = await manager.review_pull_request(repo_name, pr_number)
            
            console.print(f"\nStatus: {review.approval_status}")
            if review.comments:
                console.print("\nIssues found:")
                for comment in review.comments:
                    console.print(f"  • {comment}")
        else:
            console.print(f"[bold]Analyzing repository: {repo_name}[/bold]")
            
            # Clone and analyze
            with console.status("Cloning repository..."):
                repo = await manager.clone_repository(repo_name)
                
            analysis = await manager.analyze_codebase(repo.working_dir)
            
            console.print(f"\nLanguages: {', '.join(analysis.languages.keys())}")
            console.print(f"Dependencies: {sum(len(deps) for deps in analysis.dependencies.values())} packages")
            console.print(f"Test files: {len(analysis.tests)}")
            
    asyncio.run(_run())


@cli.command()
def clean():
    """Clean up cache and temporary files."""
    async def _run():
        state_manager = StateManager(StorageBackend(type="file"))
        deleted = await state_manager.cleanup_old_data(days=7)
        console.print(f"✓ Cleaned up {deleted} old entries")
        
        # Clear cache directory
        cache_dir = Path("./cache")
        if cache_dir.exists():
            import shutil
            shutil.rmtree(cache_dir)
            console.print("✓ Cleared cache directory")
            
    asyncio.run(_run())


if __name__ == "__main__":
    cli()