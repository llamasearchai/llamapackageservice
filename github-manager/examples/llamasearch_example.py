#!/usr/bin/env python3
"""
LlamaSearchAI Manager Usage Examples
Demonstrates all capabilities of the LlamaSearchAI repository manager
"""
import asyncio
import json
from pathlib import Path
import sys

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from src.llamasearch_manager import LlamaSearchManager


async def example_complete_workflow():
    """Example: Complete workflow for managing LlamaSearchAI repos"""
    print("=== LlamaSearchAI Complete Workflow Example ===\n")
    
    # Initialize manager
    manager = LlamaSearchManager()
    await manager.initialize()
    
    # 1. Scan organization
    print("1. Scanning LlamaSearchAI organization...")
    repos = await manager.scan_organization_repos()
    print(f"   Found {len(repos)} repositories")
    
    # Show key repositories
    key_repos = ['llamaagent', 'llamagraph', 'llama-cli', 'OpenPersona']
    print("\n   Key repositories:")
    for repo in repos:
        if repo['name'] in key_repos:
            print(f"   - {repo['name']}: {repo['description']}")
    
    # 2. Clone repositories
    print("\n2. Cloning/updating repositories...")
    clone_results = await manager.clone_all_repos()
    success_count = sum(1 for r in clone_results.values() if 'Error' not in r)
    print(f"   Successfully cloned/updated {success_count}/{len(clone_results)} repositories")
    
    # 3. Generate concatenated files
    print("\n3. Generating concatenated files...")
    output_dir = Path("output/llamasearch_example")
    concat_results = await manager.generate_concatenated_files(str(output_dir))
    print(f"   Generated {len(concat_results)} concatenated files")
    print(f"   Output directory: {output_dir}")
    
    # 4. Analyze ecosystem
    print("\n4. Analyzing LlamaSearchAI ecosystem...")
    analysis = await manager.analyze_ecosystem()
    
    print("\n   Language distribution:")
    for lang, count in list(analysis['languages'].items())[:5]:
        print(f"   - {lang}: {count} repositories")
    
    print("\n   Common dependencies:")
    for dep_type, deps in analysis['dependencies'].items():
        if deps:
            print(f"   - {dep_type}: {len(deps)} unique dependencies")
    
    # 5. Generate documentation
    print("\n5. Generating comprehensive documentation...")
    doc_dir = await manager.generate_documentation()
    print(f"   Documentation saved to: {doc_dir}")
    
    await manager.cleanup()
    print("\n=== Workflow Complete ===")


async def example_specific_repo_analysis():
    """Example: Detailed analysis of specific repositories"""
    print("=== Analyzing Specific LlamaSearchAI Repositories ===\n")
    
    manager = LlamaSearchManager()
    await manager.initialize()
    
    # Focus on core repositories
    core_repos = ['llamaagent', 'llamagraph', 'llama-cli']
    
    for repo_name in core_repos:
        print(f"\n📦 Analyzing {repo_name}...")
        
        repo_path = manager.local_repos_path / manager.org_name / repo_name
        
        if repo_path.exists():
            # Get repository metadata
            metadata = manager.repo_metadata.get(repo_name, {})
            print(f"   Stars: {metadata.get('stars', 0)}")
            print(f"   Language: {metadata.get('language', 'Unknown')}")
            print(f"   Topics: {', '.join(metadata.get('topics', []))}")
            
            # AI analysis
            ai_prompt = f"""
            Analyze the {repo_name} repository and provide:
            1. Main purpose and functionality
            2. Key technical features
            3. Integration points with other LlamaSearchAI repos
            4. Suggested improvements
            """
            
            analysis = await manager.ai_analyze_repository(str(repo_path))
            if 'analysis' in analysis:
                print(f"\n   AI Analysis:")
                print(f"   {analysis['analysis'][:500]}...")
            
            # Dependency analysis
            deps = await manager._analyze_repo_dependencies(repo_path)
            for dep_type, dep_list in deps.items():
                if dep_list:
                    print(f"\n   {dep_type.upper()} Dependencies ({len(dep_list)}):")
                    for dep in list(dep_list)[:5]:
                        print(f"   - {dep}")
    
    await manager.cleanup()


async def example_create_llamasearch_project():
    """Example: Create a new LlamaSearchAI project"""
    print("=== Creating New LlamaSearchAI Project ===\n")
    
    manager = LlamaSearchManager()
    await manager.initialize()
    
    project_name = "llama-example-service"
    
    print(f"Creating project: {project_name}")
    result = await manager.create_development_environment(project_name)
    
    if result['success']:
        print(f"\n✅ Project created successfully!")
        print(f"   Local path: {result['local_path']}")
        print(f"   Files created: {result['files_created']}")
        
        if result.get('github_url'):
            print(f"   GitHub URL: {result['github_url']}")
        
        # Show project structure
        print("\n   Project structure:")
        local_path = Path(result['local_path'])
        for item in sorted(local_path.rglob('*'))[:20]:
            if item.is_file():
                rel_path = item.relative_to(local_path)
                print(f"   - {rel_path}")
    else:
        print(f"❌ Failed to create project: {result.get('error')}")
    
    await manager.cleanup()


async def example_search_and_update():
    """Example: Search across repos and update content"""
    print("=== Search and Update Example ===\n")
    
    manager = LlamaSearchManager()
    await manager.initialize()
    
    # Example: Search for TODO comments
    print("1. Searching for TODO comments across repositories...")
    
    # This would use MCP client if servers are running
    search_query = "TODO"
    
    # Manual search for demo
    todos_found = []
    for repo_name in ['llamaagent', 'llamagraph']:
        repo_path = manager.local_repos_path / manager.org_name / repo_name
        if repo_path.exists():
            for py_file in repo_path.rglob('*.py'):
                try:
                    with open(py_file, 'r') as f:
                        for i, line in enumerate(f, 1):
                            if 'TODO' in line:
                                todos_found.append({
                                    'repo': repo_name,
                                    'file': str(py_file.relative_to(repo_path)),
                                    'line': i,
                                    'content': line.strip()
                                })
                except:
                    pass
    
    print(f"   Found {len(todos_found)} TODO items")
    for todo in todos_found[:5]:
        print(f"   - {todo['repo']}/{todo['file']}:{todo['line']}")
        print(f"     {todo['content']}")
    
    # Example: Update repository metadata
    print("\n2. Updating repository metadata...")
    
    # This is a demo - would need proper permissions
    update_data = {
        'description': 'Updated description for demo',
        'topics': ['ai', 'llm', 'search', 'agents']
    }
    
    print(f"   Would update repository with:")
    print(f"   - New description: {update_data['description']}")
    print(f"   - Topics: {', '.join(update_data['topics'])}")
    
    await manager.cleanup()


async def example_monitoring():
    """Example: Monitor repositories for changes"""
    print("=== Repository Monitoring Example ===\n")
    
    manager = LlamaSearchManager()
    await manager.initialize()
    
    print("Setting up monitoring for key repositories...")
    
    # Define what to monitor
    key_repos = ['llamaagent', 'llamagraph', 'llama-cli']
    
    # In a real scenario, this would connect to MCP servers
    print("\nWould monitor the following:")
    for repo in key_repos:
        print(f"\n📦 {repo}:")
        print("   - New commits")
        print("   - New issues")
        print("   - New pull requests")
        print("   - Release tags")
    
    print("\nMonitoring callbacks would trigger on:")
    print("- Commit: Log changes and run tests")
    print("- Issue: Analyze with AI and suggest labels")
    print("- PR: Automated code review")
    print("- Release: Generate changelog and documentation")
    
    await manager.cleanup()


async def example_batch_operations():
    """Example: Perform batch operations across repositories"""
    print("=== Batch Operations Example ===\n")
    
    manager = LlamaSearchManager()
    await manager.initialize()
    
    # Select repositories for batch operations
    target_repos = ['llamaagent', 'llamagraph', 'llama-cli']
    
    print(f"Performing batch operations on {len(target_repos)} repositories...")
    
    # 1. Batch sync
    print("\n1. Batch sync...")
    # This would actually sync
    print(f"   Would sync: {', '.join(target_repos)}")
    
    # 2. Batch analysis
    print("\n2. Batch analysis...")
    batch_results = await manager.batch_operation('analyze', target_repos)
    print(f"   Analyzed {len(batch_results)} repositories")
    
    # 3. Generate individual reports
    print("\n3. Generating individual concatenated files...")
    for repo in target_repos:
        output_file = Path(f"output/{repo}_individual.txt")
        print(f"   Would generate: {output_file}")
    
    await manager.cleanup()


# Menu system for examples
async def main():
    """Main menu for examples"""
    print("🦙 LlamaSearchAI Manager Examples")
    print("================================\n")
    
    examples = {
        '1': ('Complete Workflow', example_complete_workflow),
        '2': ('Specific Repository Analysis', example_specific_repo_analysis),
        '3': ('Create New Project', example_create_llamasearch_project),
        '4': ('Search and Update', example_search_and_update),
        '5': ('Repository Monitoring', example_monitoring),
        '6': ('Batch Operations', example_batch_operations)
    }
    
    print("Available examples:")
    for key, (name, _) in examples.items():
        print(f"{key}. {name}")
    
    choice = input("\nSelect example (1-6) or 'all' to run all: ").strip()
    
    if choice == 'all':
        for name, func in examples.values():
            print(f"\n{'=' * 60}")
            print(f"Running: {name}")
            print('=' * 60)
            await func()
            print("\nPress Enter to continue...")
            input()
    elif choice in examples:
        await examples[choice][1]()
    else:
        print("Invalid choice. Running complete workflow example...")
        await example_complete_workflow()


if __name__ == "__main__":
    asyncio.run(main())