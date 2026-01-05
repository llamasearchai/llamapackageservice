"""
Legacy UI for LlamaPackageService.

Provides a simple terminal-based UI for interactive processing.
"""

from pathlib import Path
from typing import Optional, List
from dataclasses import dataclass
import asyncio
import logging

try:
    from rich.console import Console
    from rich.panel import Panel
    from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn
    from rich.prompt import Prompt, Confirm
    from rich.table import Table
    from rich.layout import Layout
    from rich.live import Live
    from rich.text import Text
    HAS_RICH = True
except ImportError:
    HAS_RICH = False

from .config import Config
from .processors import ProcessorFactory
from .output_organizer import OutputPaths, list_output_files

logger = logging.getLogger(__name__)


@dataclass
class UIState:
    """UI state."""
    current_url: Optional[str] = None
    url_type: Optional[str] = None
    processing: bool = False
    progress: int = 0
    message: str = ""


class LlamaUI:
    """Legacy terminal UI for LlamaPackageService."""
    
    LOGO = """
    ╦  ╦  ╔═╗ ╔╦╗ ╔═╗   ╔═╗ ╔═╗ ╔═╗ ╦╔═ ╔═╗ ╔═╗ ╔═╗
    ║  ║  ╠═╣ ║║║ ╠═╣   ╠═╝ ╠═╣ ║   ╠╩╗ ╠═╣ ║ ╦ ║╣
    ╩═╝╩═╝╩ ╩ ╩ ╩ ╩ ╩   ╩   ╩ ╩ ╚═╝ ╩ ╩ ╩ ╩ ╚═╝ ╚═╝
            ╔═╗ ╔═╗ ╦═╗ ╦  ╦ ╦ ╔═╗ ╔═╗
            ╚═╗ ║╣  ╠╦╝ ╚╗╔╝ ║ ║   ║╣
            ╚═╝ ╚═╝ ╩╚═  ╚╝  ╩ ╚═╝ ╚═╝
    """
    
    def __init__(self, config: Config):
        """Create a new UI."""
        self._config = config
        self._state = UIState()
        
        if HAS_RICH:
            self._console = Console()
        else:
            self._console = None
    
    def _print(self, text: str, style: Optional[str] = None):
        """Print text with optional styling."""
        if self._console:
            self._console.print(text, style=style)
        else:
            print(text)
    
    def show_banner(self):
        """Show the application banner."""
        if self._console:
            self._console.print(Panel(
                Text(self.LOGO, justify="center", style="bold cyan"),
                title="LlamaPackageService",
                subtitle="v0.1.0",
                border_style="cyan",
            ))
        else:
            print(self.LOGO)
            print("LlamaPackageService v0.1.0")
    
    def show_menu(self) -> str:
        """Show the main menu and get user selection."""
        menu_items = [
            ("1", "Process a package URL"),
            ("2", "List processed packages"),
            ("3", "View configuration"),
            ("4", "Generate index"),
            ("5", "Quit"),
        ]
        
        if self._console:
            table = Table(title="Main Menu", show_header=False, border_style="dim")
            table.add_column("Option", style="cyan")
            table.add_column("Description")
            
            for opt, desc in menu_items:
                table.add_row(opt, desc)
            
            self._console.print(table)
            choice = Prompt.ask("Select an option", choices=["1", "2", "3", "4", "5"], default="1")
        else:
            print("\nMain Menu:")
            for opt, desc in menu_items:
                print(f"  {opt}. {desc}")
            choice = input("Select an option [1-5]: ").strip() or "1"
        
        return choice
    
    def prompt_url(self) -> str:
        """Prompt for a URL to process."""
        if self._console:
            return Prompt.ask("Enter package URL or path")
        else:
            return input("Enter package URL or path: ").strip()
    
    def show_url_type(self, url: str) -> str:
        """Show the detected URL type."""
        url_type = ProcessorFactory.detect_url_type(url)
        
        self._print(f"\nDetected URL type: [bold green]{url_type}[/bold green]" if self._console 
                   else f"\nDetected URL type: {url_type}")
        
        return url_type
    
    async def process_url(self, url: str) -> bool:
        """Process a URL with progress display."""
        self._state.current_url = url
        self._state.url_type = ProcessorFactory.detect_url_type(url)
        self._state.processing = True
        
        processor = ProcessorFactory.create_processor(url)
        
        try:
            # Validate
            self._print("\nValidating URL...", style="dim")
            await processor.validate(url)
            
            # Process with progress
            if self._console:
                with Progress(
                    SpinnerColumn(),
                    TextColumn("[bold blue]{task.description}"),
                    BarColumn(),
                    TaskProgressColumn(),
                    console=self._console,
                ) as progress:
                    task = progress.add_task("Processing...", total=100)
                    
                    # Start processing
                    async def update_progress():
                        while self._state.processing:
                            if self._state.progress < 90:
                                self._state.progress += 5
                                progress.update(task, completed=self._state.progress)
                            await asyncio.sleep(0.5)
                    
                    progress_task = asyncio.create_task(update_progress())
                    
                    try:
                        await processor.process(url, self._config.output_dir, self._config)
                        self._state.progress = 100
                        progress.update(task, completed=100)
                    finally:
                        self._state.processing = False
                        progress_task.cancel()
                        try:
                            await progress_task
                        except asyncio.CancelledError:
                            pass
            else:
                print("Processing...")
                await processor.process(url, self._config.output_dir, self._config)
                print("Done!")
            
            self._print("\n[bold green]✓ Processing completed successfully![/bold green]" if self._console
                       else "\n✓ Processing completed successfully!")
            return True
            
        except Exception as e:
            self._state.processing = False
            self._print(f"\n[bold red]✗ Error: {e}[/bold red]" if self._console
                       else f"\n✗ Error: {e}")
            return False
    
    def show_processed_packages(self):
        """Show list of processed packages."""
        paths = OutputPaths(self._config.output_dir)
        files = list_output_files(paths)
        
        if not files:
            self._print("\nNo processed packages found.", style="dim")
            return
        
        if self._console:
            table = Table(title="Processed Packages", border_style="dim")
            table.add_column("Type", style="cyan")
            table.add_column("Name")
            table.add_column("Size")
            
            for file_path in files:
                pkg_type = file_path.parent.name
                size = file_path.stat().st_size
                size_str = f"{size / 1024:.1f} KB" if size >= 1024 else f"{size} B"
                table.add_row(pkg_type, file_path.name, size_str)
            
            self._console.print(table)
        else:
            print("\nProcessed Packages:")
            for file_path in files:
                print(f"  - {file_path.parent.name}/{file_path.name}")
    
    def show_configuration(self):
        """Show current configuration."""
        if self._console:
            table = Table(title="Configuration", border_style="dim")
            table.add_column("Setting", style="cyan")
            table.add_column("Value")
            
            table.add_row("Output directory", str(self._config.output_dir))
            table.add_row("Max concurrent", str(self._config.processing.max_concurrent))
            table.add_row("Retry attempts", str(self._config.processing.retry_attempts))
            table.add_row("Timeout", f"{self._config.processing.timeout}s")
            
            self._console.print(table)
        else:
            print("\nConfiguration:")
            print(f"  Output directory: {self._config.output_dir}")
            print(f"  Max concurrent: {self._config.processing.max_concurrent}")
            print(f"  Retry attempts: {self._config.processing.retry_attempts}")
            print(f"  Timeout: {self._config.processing.timeout}s")
    
    async def generate_index(self):
        """Generate output index."""
        from .output_organizer import generate_index
        
        self._print("\nGenerating index...", style="dim")
        
        paths = OutputPaths(self._config.output_dir)
        generate_index(paths)
        
        self._print("[bold green]✓ Index generated![/bold green]" if self._console
                   else "✓ Index generated!")
    
    async def run(self):
        """Run the interactive UI loop."""
        self.show_banner()
        
        while True:
            try:
                choice = self.show_menu()
                
                if choice == "1":
                    url = self.prompt_url()
                    if url:
                        self.show_url_type(url)
                        if self._console:
                            if Confirm.ask("Process this URL?"):
                                await self.process_url(url)
                        else:
                            confirm = input("Process this URL? [Y/n]: ").strip().lower()
                            if confirm != "n":
                                await self.process_url(url)
                
                elif choice == "2":
                    self.show_processed_packages()
                
                elif choice == "3":
                    self.show_configuration()
                
                elif choice == "4":
                    await self.generate_index()
                
                elif choice == "5":
                    self._print("\nGoodbye! 🦙", style="bold cyan")
                    break
                
            except KeyboardInterrupt:
                self._print("\n\nInterrupted. Goodbye! 🦙", style="bold cyan")
                break


async def run_ui(config: Config):
    """Run the legacy UI."""
    ui = LlamaUI(config)
    await ui.run()
