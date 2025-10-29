#!/usr/bin/env python3
"""
Example: Using GitHub Repository Manager as a library
"""
import asyncio
from pathlib import Path
import sys

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from src.core.repo_manager import GitHubRepoManager
from src.core.ollama_interface import OllamaInterface


async def example_usage():
    """Demonstrate programmatic usage of the GitHub Repository Manager"""
    
    # Initialize the manager
    manager = GitHubRepoManager()
    await manager.initialize()
    
    print("=== GitHub Repository Manager Example ===\n")
    
    # 1. List local repositories
    print("1. Scanning local repositories...")
    repos = manager.scan_local_repositories()
    print(f"Found {len(repos)} repositories:")
    for repo in repos[:3]:
        print(f"  - {repo['name']} ({repo['current_branch']})")
    
    # 2. Analyze a repository with AI
    if repos:
        print(f"\n2. Analyzing repository: {repos[0]['name']}...")
        analysis = await manager.ai_analyze_repository(repos[0]['path'])
        if 'analysis' in analysis:
            print("AI Analysis Summary:")
            print(analysis['analysis'][:500] + "...")
    
    # 3. Use Ollama directly for custom analysis
    print("\n3. Custom Ollama analysis...")
    ollama = manager.ollama
    result = await ollama.analyze_code_with_tools(
        ".",
        "What are the main Python files in this project and what do they do?"
    )
    print(f"Analysis complete. Tools used: {result['tools_used']}")
    
    # 4. GitHub API operations
    print("\n4. GitHub API example...")
    try:
        user = manager.github.get_user()
        print(f"Connected as: {user.login}")
        print(f"Public repos: {user.public_repos}")
        
        # List recent repositories
        recent_repos = list(user.get_repos())[:3]
        print("Recent repositories:")
        for repo in recent_repos:
            print(f"  - {repo.name} ⭐ {repo.stargazers_count}")
    except Exception as e:
        print(f"GitHub API error: {e}")
    
    # 5. Batch operations
    print("\n5. Batch operation example...")
    if len(repos) > 1:
        repo_names = [r['name'] for r in repos[:2]]
        results = await manager.batch_operation('analyze', repo_names)
        print(f"Analyzed {len(results)} repositories")
    
    # 6. Search functionality
    print("\n6. Code search example...")
    try:
        search_results = await manager.mcp_client.search_code(
            "TODO",
            repo=repos[0]['name'] if repos else None
        )
        print(f"Found {len(search_results)} TODO items")
    except Exception as e:
        print(f"Search not available (MCP server may be offline): {e}")
    
    # Cleanup
    await manager.cleanup()
    print("\n=== Example Complete ===")


async def custom_workflow_example():
    """Example of a custom workflow"""
    manager = GitHubRepoManager()
    await manager.initialize()
    
    print("\n=== Custom Workflow: Security Audit ===\n")
    
    # Define a security audit workflow
    repos = manager.scan_local_repositories()
    
    for repo in repos[:2]:  # Limit to first 2 repos
        print(f"\nAuditing: {repo['name']}")
        
        # Use AI to check for security issues
        security_prompt = """
        Analyze this repository for potential security issues:
        1. Hardcoded credentials or API keys
        2. Vulnerable dependencies
        3. Security best practices violations
        4. Exposed sensitive data
        
        Provide a brief security assessment.
        """
        
        result = await manager.ollama.analyze_code_with_tools(
            repo['path'],
            security_prompt
        )
        
        print("Security Assessment:")
        print(result['analysis'])
        print("-" * 50)
    
    await manager.cleanup()


async def mcp_integration_example():
    """Example of direct MCP server integration"""
    from src.core.mcp_client import MCPClient
    
    print("\n=== MCP Server Integration Example ===\n")
    
    client = MCPClient()
    
    # Connect to servers
    await client.connect_servers([
        {'name': 'github', 'host': 'localhost', 'port': 3001}
    ])
    
    # List repositories via MCP
    print("1. Listing repositories via MCP...")
    try:
        repos = await client.list_repositories()
        print(f"Found {len(repos)} repositories")
        for repo in repos[:3]:
            print(f"  - {repo['name']} ⭐ {repo.get('stars', 0)}")
    except Exception as e:
        print(f"MCP error: {e}")
        print("Make sure MCP servers are running: ./start-mcp-servers.sh")
    
    # Get repository insights
    if repos:
        print(f"\n2. Getting insights for: {repos[0]['full_name']}")
        try:
            insights = await client.get_repository_insights(repos[0]['full_name'])
            print(f"  Stars: {insights['basic_info']['stars']}")
            print(f"  Language: {insights['basic_info']['language']}")
            print(f"  Open Issues: {insights['basic_info']['open_issues']}")
        except Exception as e:
            print(f"Failed to get insights: {e}")
    
    await client.disconnect()


if __name__ == "__main__":
    print("GitHub Repository Manager - Integration Examples\n")
    print("Choose an example:")
    print("1. Basic usage")
    print("2. Custom security workflow")
    print("3. MCP server integration")
    
    choice = input("\nEnter choice (1-3): ").strip()
    
    if choice == "1":
        asyncio.run(example_usage())
    elif choice == "2":
        asyncio.run(custom_workflow_example())
    elif choice == "3":
        asyncio.run(mcp_integration_example())
    else:
        print("Invalid choice")
        asyncio.run(example_usage())