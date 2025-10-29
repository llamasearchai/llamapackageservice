#!/usr/bin/env python3
"""
System test script for GitHub Repository Manager
"""
import asyncio
import os
import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent))

from src.core.repo_manager import GitHubRepoManager
from src.utils.logger import setup_logger

logger = setup_logger(__name__)


async def test_system():
    """Run system tests"""
    print("🧪 GitHub Repository Manager System Test")
    print("=" * 50)
    
    # Check environment
    print("\n1️⃣ Checking environment...")
    if not os.getenv('GITHUB_TOKEN'):
        print("❌ GITHUB_TOKEN not set")
        print("   Set it with: export GITHUB_TOKEN='your_token'")
        return False
    else:
        print("✅ GitHub token found")
    
    # Initialize manager
    print("\n2️⃣ Initializing manager...")
    try:
        manager = GitHubRepoManager()
        await manager.initialize()
        print("✅ Manager initialized")
    except Exception as e:
        print(f"❌ Failed to initialize: {e}")
        return False
    
    # Scan repositories
    print("\n3️⃣ Scanning local repositories...")
    try:
        repos = manager.scan_local_repositories()
        print(f"✅ Found {len(repos)} repositories")
        for repo in repos[:3]:  # Show first 3
            print(f"   - {repo['name']} ({repo['current_branch']})")
    except Exception as e:
        print(f"❌ Failed to scan: {e}")
    
    # Test Ollama
    print("\n4️⃣ Testing Ollama integration...")
    try:
        result = await manager.ollama.analyze_text("Hello, test the system")
        print("✅ Ollama responding")
    except Exception as e:
        print(f"❌ Ollama error: {e}")
        print("   Make sure Ollama is running: ollama serve")
    
    # Test MCP client
    print("\n5️⃣ Testing MCP client...")
    try:
        status = await manager.mcp_client.get_server_status('github')
        if status['status'] == 'connected':
            print("✅ MCP GitHub server connected")
        else:
            print("⚠️  MCP GitHub server not connected")
            print("   Run ./start-mcp-servers.sh in another terminal")
    except Exception as e:
        print(f"⚠️  MCP client error: {e}")
    
    # Test GitHub API
    print("\n6️⃣ Testing GitHub API...")
    try:
        user = manager.github.get_user()
        print(f"✅ Connected as: {user.login}")
        print(f"   Public repos: {user.public_repos}")
    except Exception as e:
        print(f"❌ GitHub API error: {e}")
    
    # Cleanup
    await manager.cleanup()
    
    print("\n" + "=" * 50)
    print("✅ System test complete!")
    print("\nNext steps:")
    print("1. Start MCP servers: ./start-mcp-servers.sh")
    print("2. Run CLI: ./github-manager --help")
    print("3. Start dashboard: ./start-dashboard.sh")
    
    return True


if __name__ == "__main__":
    success = asyncio.run(test_system())
    sys.exit(0 if success else 1)