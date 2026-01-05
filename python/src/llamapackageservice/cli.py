"""
CLI interface for the LlamaPackageService.

This module provides the command-line interface matching the Rust implementation.
"""

import asyncio
import sys
from pathlib import Path
from typing import Optional

import click
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.progress import Progress, SpinnerColumn, TextColumn

from .config import Config
from .processors import ProcessorFactory
from .output_organizer import list_output_files, organize_output, generate_index
from .utils import normalize_url_or_path


console = Console()


def print_banner():
    """Print a colorful banner at the start of the CLI."""
    banner = """
         __      _                     
       /\\ \\__  /\\ \\__                
  _____\\ \\ ,_\\/  \\ ,_\\   LlamaSearch    
_______\\ \\ \\/____\\ \\ \\/___ Package Analyzer
         \\ \\__\\   \\ \\__\\                
    """
    console.print(Panel(banner, style="bold blue", title="[green]Welcome[/green]"))


def print_help_info():
    """Print help information."""
    console.print("\n[yellow]Commands:[/yellow]")
    console.print("  [green]q[/green]          - Quit the application")
    console.print("  [green]h[/green]          - Show this help")
    console.print("  [green]examples[/green]   - Show example URLs")
    console.print("\n[yellow]Simply paste or type any URL or local path from:[/yellow]")
    console.print("  - Local files and directories")
    console.print("  - GitHub repositories")
    console.print("  - PyPI packages")
    console.print("  - NPM packages")
    console.print("  - Rust crates")
    console.print("  - Go packages")


def print_examples():
    """Print example URLs."""
    console.print("\n[yellow]Example inputs:[/yellow]")
    
    console.print("  [bold white]Local Files/Directories:[/bold white]")
    console.print("    ./my-project")
    console.print("    /Users/username/code/my-app")
    console.print("    ../other-project")
    console.print("    ./src/main.rs")
    
    console.print("\n  [bold white]GitHub:[/bold white]")
    console.print("    https://github.com/rust-lang/rust")
    console.print("    https://github.com/microsoft/typescript")
    
    console.print("\n  [bold white]PyPI:[/bold white]")
    console.print("    https://pypi.org/project/requests")
    console.print("    pip install numpy")
    
    console.print("\n  [bold white]NPM:[/bold white]")
    console.print("    https://www.npmjs.com/package/react")
    console.print("    npm install express")
    
    console.print("\n  [bold white]Rust:[/bold white]")
    console.print("    https://crates.io/crates/tokio")
    console.print("    https://docs.rs/serde")
    
    console.print("\n  [bold white]Go:[/bold white]")
    console.print("    https://pkg.go.dev/github.com/gofiber/fiber")


async def process_url(url: str, config: Config) -> None:
    """Process a URL directly."""
    # Normalize the input
    normalized = normalize_url_or_path(url)
    url_type = ProcessorFactory.detect_url_type(normalized)
    
    console.print(f"[green]Processing URL:[/green] [white]{normalized}[/white] ([cyan]{url_type}[/cyan])")
    
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
    ) as progress:
        task = progress.add_task(f"Processing {normalized}...", total=None)
        
        try:
            processor = ProcessorFactory.create_processor(normalized)
            await processor.process(normalized, config.output_dir, config)
            
            progress.update(task, description="[green]✓[/green] Processing completed")
            console.print(f"[green][SUCCESS][/green] Processing completed successfully")
            console.print(f"[blue][SAVED][/blue] Results saved to {config.output_dir}")
            
        except Exception as e:
            progress.update(task, description="[red]✗[/red] Processing failed")
            console.print(f"[red][ERROR][/red] Error: {e}")
            raise


async def run_simple_interactive(config: Config) -> None:
    """Run a simple interactive command-line interface."""
    console.print("\n[bright_green bold]Welcome to The Llama Package Service[/bright_green bold]")
    console.print("[bright_blue]A tool for analyzing and documenting software packages[/bright_blue]")
    console.print("[bright_yellow]================================================[/bright_yellow]\n")
    
    # Show examples of supported URL types
    console.print("[yellow]Examples of supported inputs:[/yellow]")
    console.print("  [bold white]Local Directory:[/bold white] ./my-project")
    console.print("  [bold white]Local Directory:[/bold white] /path/to/my/code")
    console.print("  [bold white]Local File:[/bold white] ./README.md")
    console.print("  [bold white]GitHub:[/bold white] https://github.com/username/repository")
    console.print("  [bold white]PyPI:[/bold white] https://pypi.org/project/package-name")
    console.print("  [bold white]NPM:[/bold white] https://www.npmjs.com/package/package-name")
    console.print("  [bold white]Rust:[/bold white] https://crates.io/crates/crate-name")
    console.print("  [bold white]Go:[/bold white] https://pkg.go.dev/github.com/username/package")
    console.print("  [bold white]You can also use commands like 'pip install package-name'[/bold white]")
    
    while True:
        console.print("\n[cyan]Enter a URL or local path to process (or 'q' to quit, 'h' for help):[/cyan]")
        
        try:
            user_input = input("> ").strip()
        except (KeyboardInterrupt, EOFError):
            console.print("\n[green]Thank you for using The Llama Package Service![/green]")
            break
        
        if not user_input:
            continue
        
        # Handle special commands
        lower_input = user_input.lower()
        if lower_input in ("q", "quit", "exit"):
            console.print("[green]Thank you for using The Llama Package Service![/green]")
            break
        elif lower_input in ("h", "help", "?"):
            print_help_info()
            continue
        elif lower_input == "examples":
            print_examples()
            continue
        
        # Try to process the URL
        try:
            await process_url(user_input, config)
            console.print("\n[green]Processing completed successfully.[/green]")
            console.print("[cyan]Enter another URL or 'q' to quit.[/cyan]")
        except Exception as e:
            console.print(f"\n[red][ERROR][/red] Error: {e}")
            console.print("[cyan]Try a different URL or enter 'examples' to see supported URL formats.[/cyan]")


async def run_legacy_ui(config: Config) -> None:
    """Run the legacy UI with menu-based interface."""
    from .ui import LlamaUI
    ui = LlamaUI()
    await ui.run(config)


async def async_main(
    url: Optional[str],
    output: Optional[Path],
    legacy_ui: bool,
    index: bool,
    api: bool,
    api_port: int,
) -> None:
    """Async main entry point."""
    # Create configuration
    output_dir = output or Path("./output")
    config = Config(output_dir=output_dir)
    
    # Create output directory if it doesn't exist
    output_dir.mkdir(parents=True, exist_ok=True)
    
    if api:
        # Start the API server
        from .api import start_server
        await start_server(config, port=api_port)
        return
    
    # Process according to mode
    if url:
        # Process URL directly if provided
        await process_url(url, config)
    elif legacy_ui:
        # Use the legacy UI if explicitly requested
        await run_legacy_ui(config)
    else:
        # Use the new simple interactive mode
        await run_simple_interactive(config)
    
    # Organize output files into appropriate directories
    try:
        organize_output(output_dir)
    except Exception as e:
        console.print(f"[yellow]Warning:[/yellow] Failed to organize output files: {e}")
    
    # Generate index if requested
    if index:
        try:
            generate_index(output_dir)
        except Exception as e:
            console.print(f"[red]Failed to generate index:[/red] {e}")
    
    # Display the output files
    list_output_files(output_dir)


@click.command()
@click.option("--url", "-u", help="URL to process directly (skip interactive mode)")
@click.option("--output", "-o", type=click.Path(path_type=Path), help="Output directory")
@click.option("--legacy-ui", "-l", is_flag=True, help="Run in legacy interactive mode with UI")
@click.option("--index", "-x", is_flag=True, help="Generate index file after processing")
@click.option("--api", "-a", is_flag=True, help="Start the REST API server")
@click.option("--api-port", default=8000, help="Port for the REST API server")
@click.version_option(version="0.1.0", prog_name="llamapackageservice")
def main(
    url: Optional[str],
    output: Optional[Path],
    legacy_ui: bool,
    index: bool,
    api: bool,
    api_port: int,
) -> None:
    """
    LlamaPackageService - A tool for analyzing and documenting software packages.
    
    Process repositories from GitHub, PyPI, NPM, crates.io, and local directories.
    """
    try:
        # Try to limit memory usage
        try:
            from . import limit_memory_usage
            limit_memory_usage()
        except Exception:
            pass
        
        asyncio.run(async_main(url, output, legacy_ui, index, api, api_port))
    except KeyboardInterrupt:
        console.print("\n[yellow]Interrupted by user[/yellow]")
        sys.exit(130)
    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
