"""
Main entry point for running LlamaPackageService as a module.

Usage:
    python -m llamapackageservice [OPTIONS] [URL]
"""

from .cli import main

if __name__ == "__main__":
    main()
