#!/usr/bin/env python3
"""
LlamaSearchAI GitHub Organization Manager
Complete management system for llamasearchai repositories
"""
import asyncio
import json
import os
import subprocess
from pathlib import Path
from typing import Dict, List, Optional, Any, Set
from datetime import datetime
import git
from github import Github, GithubException
import aiofiles
import yaml
import shutil
from collections import defaultdict
import re

from .core.repo_manager import GitHubRepoManager
from .core.mcp_client import MCPClient
from .core.ollama_interface import OllamaInterface
from .utils.logger import setup_logger

logger = setup_logger(__name__)


class LlamaSearchManager(GitHubRepoManager):
    """Enhanced manager for LlamaSearchAI organization repositories"""
    
    def __init__(self, config_path: str = "config/llamasearch_config.yaml"):
        super().__init__(config_path)
        self.org_name = "llamasearchai"
        self.organization = None
        self.repo_metadata = {}
        self.concatenation_rules = self._load_concatenation_rules()
        
    async def initialize(self):
        """Initialize with LlamaSearchAI specific setup"""
        await super().initialize()
        
        try:
            self.organization = self.github.get_organization(self.org_name)
            logger.info(f"Connected to {self.org_name} organization")
        except GithubException:
            # Try as user if org access fails
            logger.warning(f"Could not access as organization, trying as user")
            self.organization = self.github.get_user(self.org_name)
        
        # Scan organization repositories
        await self.scan_organization_repos()
    
    def _load_concatenation_rules(self) -> Dict[str, Any]:
        """Load rules for concatenating repository files"""
        return {
            'python': {
                'extensions': ['.py', '.pyx', '.pyi'],
                'ignore_patterns': ['__pycache__', '*.pyc', '.pytest_cache', 'venv', 'env'],
                'priority_files': ['setup.py', 'pyproject.toml', 'requirements.txt', 'README.md'],
                'include_docs': True
            },
            'rust': {
                'extensions': ['.rs', '.toml'],
                'ignore_patterns': ['target/', 'Cargo.lock'],
                'priority_files': ['Cargo.toml', 'src/lib.rs', 'src/main.rs', 'README.md'],
                'include_docs': True
            },
            'javascript': {
                'extensions': ['.js', '.jsx', '.ts', '.tsx'],
                'ignore_patterns': ['node_modules/', 'dist/', 'build/'],
                'priority_files': ['package.json', 'index.js', 'index.ts', 'README.md'],
                'include_docs': True
            },
            'documentation': {
                'extensions': ['.md', '.rst', '.txt'],
                'ignore_patterns': [],
                'priority_files': ['README.md', 'CONTRIBUTING.md', 'LICENSE'],
                'include_docs': True
            }
        }
    
    async def scan_organization_repos(self) -> List[Dict]:
        """Scan all LlamaSearchAI organization repositories"""
        logger.info(f"Scanning {self.org_name} repositories...")
        
        org_repos = []
        
        try:
            # Get all organization repositories
            for repo in self.organization.get_repos():
                repo_info = {
                    'name': repo.name,
                    'full_name': repo.full_name,
                    'description': repo.description,
                    'language': repo.language,
                    'stars': repo.stargazers_count,
                    'forks': repo.forks_count,
                    'private': repo.private,
                    'url': repo.html_url,
                    'clone_url': repo.clone_url,
                    'default_branch': repo.default_branch,
                    'created_at': repo.created_at.isoformat(),
                    'updated_at': repo.updated_at.isoformat(),
                    'topics': repo.get_topics(),
                    'size': repo.size,
                    'archived': repo.archived
                }
                org_repos.append(repo_info)
                self.repo_metadata[repo.name] = repo_info
                
            logger.info(f"Found {len(org_repos)} repositories in {self.org_name}")
            
            # Save metadata
            await self._save_repo_metadata(org_repos)
            
        except Exception as e:
            logger.error(f"Error scanning organization: {e}")
        
        return org_repos
    
    async def clone_all_repos(self, target_dir: Optional[str] = None) -> Dict[str, str]:
        """Clone all organization repositories locally"""
        if not target_dir:
            target_dir = self.local_repos_path / self.org_name
        else:
            target_dir = Path(target_dir)
        
        target_dir.mkdir(parents=True, exist_ok=True)
        
        results = {}
        
        for repo_name, repo_info in self.repo_metadata.items():
            local_path = target_dir / repo_name
            
            try:
                if local_path.exists():
                    # Update existing repo
                    logger.info(f"Updating {repo_name}...")
                    repo = git.Repo(local_path)
                    origin = repo.remote('origin')
                    origin.pull()
                    results[repo_name] = f"Updated at {local_path}"
                else:
                    # Clone new repo
                    logger.info(f"Cloning {repo_name}...")
                    git.Repo.clone_from(repo_info['clone_url'], local_path)
                    results[repo_name] = f"Cloned to {local_path}"
                    
            except Exception as e:
                logger.error(f"Failed to clone/update {repo_name}: {e}")
                results[repo_name] = f"Error: {e}"
        
        return results
    
    async def generate_concatenated_files(self, output_dir: Optional[str] = None) -> Dict[str, Path]:
        """Generate concatenated text files for all repositories"""
        if not output_dir:
            output_dir = Path("output") / self.org_name / datetime.now().strftime("%Y%m%d_%H%M%S")
        else:
            output_dir = Path(output_dir)
        
        output_dir.mkdir(parents=True, exist_ok=True)
        
        results = {}
        
        # Generate individual repo files
        for repo_name in self.repo_metadata:
            repo_path = self.local_repos_path / self.org_name / repo_name
            
            if repo_path.exists():
                try:
                    output_file = await self._concatenate_repository(
                        repo_path, 
                        output_dir / f"{repo_name}_concatenated.txt"
                    )
                    results[repo_name] = output_file
                    logger.info(f"Generated concatenated file for {repo_name}")
                except Exception as e:
                    logger.error(f"Failed to concatenate {repo_name}: {e}")
                    results[repo_name] = None
        
        # Generate master concatenated file
        master_file = output_dir / f"{self.org_name}_all_repos_master.txt"
        await self._generate_master_file(results, master_file)
        
        # Generate index and summary
        await self._generate_index_and_summary(results, output_dir)
        
        return results
    
    async def _concatenate_repository(self, repo_path: Path, output_file: Path) -> Path:
        """Concatenate all relevant files from a repository"""
        repo_name = repo_path.name
        language = self.repo_metadata.get(repo_name, {}).get('language', 'python').lower()
        
        # Get concatenation rules for the language
        rules = self.concatenation_rules.get(language, self.concatenation_rules['python'])
        
        content_parts = []
        
        # Add header
        header = f"""
{'=' * 80}
Repository: {repo_name}
Organization: {self.org_name}
Language: {language}
Generated: {datetime.now().isoformat()}
{'=' * 80}

"""
        content_parts.append(header)
        
        # Add metadata
        if repo_name in self.repo_metadata:
            metadata = f"""
## Repository Metadata
- Description: {self.repo_metadata[repo_name].get('description', 'N/A')}
- Stars: {self.repo_metadata[repo_name].get('stars', 0)}
- Forks: {self.repo_metadata[repo_name].get('forks', 0)}
- Created: {self.repo_metadata[repo_name].get('created_at', 'N/A')}
- Updated: {self.repo_metadata[repo_name].get('updated_at', 'N/A')}
- Topics: {', '.join(self.repo_metadata[repo_name].get('topics', []))}

"""
            content_parts.append(metadata)
        
        # Process priority files first
        for priority_file in rules['priority_files']:
            file_path = repo_path / priority_file
            if file_path.exists():
                content_parts.append(f"\n{'#' * 80}\n# File: {priority_file}\n{'#' * 80}\n\n")
                try:
                    async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = await f.read()
                        content_parts.append(content)
                        content_parts.append("\n\n")
                except Exception as e:
                    content_parts.append(f"Error reading {priority_file}: {e}\n\n")
        
        # Process all other files
        files_processed = 0
        total_size = 0
        file_list = []
        
        for ext in rules['extensions']:
            for file_path in repo_path.rglob(f'*{ext}'):
                # Skip ignored patterns
                if any(pattern in str(file_path) for pattern in rules['ignore_patterns']):
                    continue
                
                # Skip if already processed as priority file
                if file_path.name in rules['priority_files']:
                    continue
                
                # Skip files larger than 1MB
                if file_path.stat().st_size > 1024 * 1024:
                    continue
                
                relative_path = file_path.relative_to(repo_path)
                file_list.append(str(relative_path))
                
                content_parts.append(f"\n{'#' * 80}\n# File: {relative_path}\n{'#' * 80}\n\n")
                
                try:
                    async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = await f.read()
                        content_parts.append(content)
                        content_parts.append("\n\n")
                        files_processed += 1
                        total_size += len(content)
                except Exception as e:
                    content_parts.append(f"Error reading {relative_path}: {e}\n\n")
        
        # Add summary
        summary = f"""
{'=' * 80}
## Processing Summary
- Files processed: {files_processed}
- Total size: {total_size:,} characters
- File types: {', '.join(set(Path(f).suffix for f in file_list if Path(f).suffix))}
{'=' * 80}

## File List:
{chr(10).join(f'- {f}' for f in sorted(file_list))}
"""
        content_parts.append(summary)
        
        # Write concatenated content
        async with aiofiles.open(output_file, 'w', encoding='utf-8') as f:
            await f.write(''.join(content_parts))
        
        return output_file
    
    async def _generate_master_file(self, results: Dict[str, Path], master_file: Path):
        """Generate a master file containing all repositories"""
        async with aiofiles.open(master_file, 'w', encoding='utf-8') as master:
            # Write header
            header = f"""
{'=' * 100}
LLAMASEARCHAI MASTER REPOSITORY CONCATENATION
Organization: {self.org_name}
Total Repositories: {len(results)}
Generated: {datetime.now().isoformat()}
{'=' * 100}

## Table of Contents
"""
            await master.write(header)
            
            # Write TOC
            for i, (repo_name, file_path) in enumerate(results.items(), 1):
                if file_path:
                    await master.write(f"{i}. {repo_name}\n")
            
            await master.write("\n" + "=" * 100 + "\n\n")
            
            # Append each repository's content
            for repo_name, file_path in results.items():
                if file_path and file_path.exists():
                    await master.write(f"\n{'#' * 100}\n")
                    await master.write(f"# REPOSITORY: {repo_name}\n")
                    await master.write(f"{'#' * 100}\n\n")
                    
                    async with aiofiles.open(file_path, 'r', encoding='utf-8') as f:
                        content = await f.read()
                        await master.write(content)
                    
                    await master.write("\n\n")
    
    async def _generate_index_and_summary(self, results: Dict[str, Path], output_dir: Path):
        """Generate index and AI-powered summary"""
        index_file = output_dir / "index.yaml"
        summary_file = output_dir / "AI_SUMMARY.md"
        
        # Generate index
        index_data = {
            'organization': self.org_name,
            'generated': datetime.now().isoformat(),
            'total_repositories': len(results),
            'repositories': {}
        }
        
        for repo_name, file_path in results.items():
            if file_path and file_path.exists():
                file_stats = file_path.stat()
                index_data['repositories'][repo_name] = {
                    'file': str(file_path.name),
                    'size': file_stats.st_size,
                    'metadata': self.repo_metadata.get(repo_name, {})
                }
        
        # Save index
        async with aiofiles.open(index_file, 'w') as f:
            await f.write(yaml.dump(index_data, default_flow_style=False))
        
        # Generate AI summary
        logger.info("Generating AI-powered summary...")
        
        summary_content = f"""# LlamaSearchAI Organization Summary

Generated: {datetime.now().isoformat()}

## Organization Overview

The LlamaSearchAI organization contains {len(results)} repositories focused on building AI-powered search and agent systems.

## Repository Analysis

"""
        
        # Analyze each repository with AI
        for repo_name in list(results.keys())[:5]:  # Limit to first 5 for summary
            if repo_name in self.repo_metadata:
                repo_info = self.repo_metadata[repo_name]
                
                # Use Ollama to analyze
                analysis_prompt = f"""
                Analyze this repository and provide a brief summary:
                Name: {repo_name}
                Description: {repo_info.get('description', 'N/A')}
                Language: {repo_info.get('language', 'Unknown')}
                Topics: {', '.join(repo_info.get('topics', []))}
                
                Provide a 2-3 sentence summary of its purpose and key features.
                """
                
                try:
                    analysis = await self.ollama.analyze_text(analysis_prompt)
                    summary_content += f"### {repo_name}\n{analysis}\n\n"
                except Exception as e:
                    logger.error(f"Failed to analyze {repo_name}: {e}")
                    summary_content += f"### {repo_name}\n{repo_info.get('description', 'No description available')}\n\n"
        
        # Add architecture overview
        summary_content += """
## Architecture Overview

Based on the repository analysis, the LlamaSearchAI ecosystem consists of:

1. **Core Agent Framework** (llamaagent) - Production-ready autonomous agents
2. **Knowledge Graph System** (llamagraph) - Entity extraction and graph building
3. **Unified CLI** (llama-cli) - Command-line interface for all services
4. **Personality System** (OpenPersona) - Multi-agent personality capabilities
5. **Meta-Search** (llama-metasearch) - Aggregated search across sources

These components work together to provide a comprehensive AI-powered search and agent platform.
"""
        
        # Save summary
        async with aiofiles.open(summary_file, 'w') as f:
            await f.write(summary_content)
        
        logger.info(f"Generated index at: {index_file}")
        logger.info(f"Generated summary at: {summary_file}")
    
    async def update_repository(self, repo_name: str, updates: Dict[str, Any]) -> Dict[str, Any]:
        """Update repository with new content or settings"""
        try:
            repo = self.organization.get_repo(repo_name)
            
            results = {'success': True, 'updates': []}
            
            # Update description
            if 'description' in updates:
                repo.edit(description=updates['description'])
                results['updates'].append('description')
            
            # Update topics
            if 'topics' in updates:
                repo.replace_topics(updates['topics'])
                results['updates'].append('topics')
            
            # Update files
            if 'files' in updates:
                for file_path, content in updates['files'].items():
                    try:
                        # Get current file SHA if exists
                        try:
                            current_file = repo.get_contents(file_path)
                            sha = current_file.sha
                        except:
                            sha = None
                        
                        # Update or create file
                        if sha:
                            repo.update_file(
                                path=file_path,
                                message=f"Update {file_path}",
                                content=content,
                                sha=sha
                            )
                        else:
                            repo.create_file(
                                path=file_path,
                                message=f"Create {file_path}",
                                content=content
                            )
                        
                        results['updates'].append(f'file:{file_path}')
                        
                    except Exception as e:
                        results['errors'] = results.get('errors', [])
                        results['errors'].append(f"Failed to update {file_path}: {e}")
            
            # Update settings
            if 'settings' in updates:
                settings = updates['settings']
                repo.edit(
                    has_wiki=settings.get('has_wiki', repo.has_wiki),
                    has_issues=settings.get('has_issues', repo.has_issues),
                    has_projects=settings.get('has_projects', repo.has_projects)
                )
                results['updates'].append('settings')
            
            return results
            
        except Exception as e:
            return {'success': False, 'error': str(e)}
    
    async def analyze_ecosystem(self) -> Dict[str, Any]:
        """Analyze the entire LlamaSearchAI ecosystem"""
        logger.info("Analyzing LlamaSearchAI ecosystem...")
        
        analysis = {
            'total_repos': len(self.repo_metadata),
            'languages': defaultdict(int),
            'topics': defaultdict(int),
            'activity': {},
            'dependencies': defaultdict(set),
            'architecture': {}
        }
        
        # Analyze each repository
        for repo_name, repo_info in self.repo_metadata.items():
            # Language distribution
            if repo_info.get('language'):
                analysis['languages'][repo_info['language']] += 1
            
            # Topic analysis
            for topic in repo_info.get('topics', []):
                analysis['topics'][topic] += 1
            
            # Check local repository for detailed analysis
            local_path = self.local_repos_path / self.org_name / repo_name
            if local_path.exists():
                # Analyze dependencies
                deps = await self._analyze_repo_dependencies(local_path)
                for dep_type, dep_list in deps.items():
                    analysis['dependencies'][dep_type].update(dep_list)
        
        # Convert sets to lists for JSON serialization
        analysis['dependencies'] = {
            k: list(v) for k, v in analysis['dependencies'].items()
        }
        
        # AI-powered architecture analysis
        architecture_prompt = f"""
        Analyze the LlamaSearchAI ecosystem architecture based on these repositories:
        {json.dumps([{
            'name': name,
            'description': info.get('description', ''),
            'language': info.get('language', '')
        } for name, info in list(self.repo_metadata.items())[:10]], indent=2)}
        
        Provide insights on:
        1. How the repositories work together
        2. The overall system architecture
        3. Key integration points
        4. Potential improvements
        """
        
        try:
            architecture_analysis = await self.ollama.analyze_text(architecture_prompt)
            analysis['architecture']['ai_analysis'] = architecture_analysis
        except Exception as e:
            logger.error(f"Failed to analyze architecture: {e}")
        
        return analysis
    
    async def _analyze_repo_dependencies(self, repo_path: Path) -> Dict[str, Set[str]]:
        """Analyze dependencies in a repository"""
        deps = defaultdict(set)
        
        # Python dependencies
        requirements_files = ['requirements.txt', 'requirements-dev.txt', 'requirements-test.txt']
        for req_file in requirements_files:
            req_path = repo_path / req_file
            if req_path.exists():
                try:
                    with open(req_path, 'r') as f:
                        for line in f:
                            line = line.strip()
                            if line and not line.startswith('#'):
                                dep = line.split('==')[0].split('>=')[0].split('<=')[0].strip()
                                deps['python'].add(dep)
                except Exception as e:
                    logger.error(f"Error reading {req_file}: {e}")
        
        # Package.json for Node.js
        package_json = repo_path / 'package.json'
        if package_json.exists():
            try:
                with open(package_json, 'r') as f:
                    data = json.load(f)
                    for dep_type in ['dependencies', 'devDependencies']:
                        if dep_type in data:
                            deps['npm'].update(data[dep_type].keys())
            except Exception as e:
                logger.error(f"Error reading package.json: {e}")
        
        # Cargo.toml for Rust
        cargo_toml = repo_path / 'Cargo.toml'
        if cargo_toml.exists():
            try:
                import toml
                with open(cargo_toml, 'r') as f:
                    data = toml.load(f)
                    if 'dependencies' in data:
                        deps['rust'].update(data['dependencies'].keys())
            except Exception as e:
                logger.error(f"Error reading Cargo.toml: {e}")
        
        return dict(deps)
    
    async def create_development_environment(self, project_name: str) -> Dict[str, Any]:
        """Create a complete development environment for LlamaSearchAI projects"""
        logger.info(f"Creating development environment for {project_name}...")
        
        dev_path = self.local_repos_path / "development" / project_name
        dev_path.mkdir(parents=True, exist_ok=True)
        
        # Analyze existing repos to determine best practices
        templates = await self._analyze_project_templates()
        
        # Create project structure based on analysis
        structure = {
            'src': {
                '__init__.py': '"""LlamaSearchAI Project"""',
                'main.py': templates.get('main.py', ''),
                'config.py': templates.get('config.py', ''),
                'utils.py': templates.get('utils.py', '')
            },
            'tests': {
                '__init__.py': '',
                'test_main.py': templates.get('test_main.py', '')
            },
            'docs': {
                'README.md': f'# {project_name}\n\nA LlamaSearchAI project.',
                'API.md': '# API Documentation'
            },
            '.github': {
                'workflows': {
                    'test.yml': templates.get('github_workflow', '')
                }
            },
            'requirements.txt': templates.get('requirements', ''),
            'setup.py': templates.get('setup.py', ''),
            '.gitignore': templates.get('gitignore', ''),
            'Dockerfile': templates.get('dockerfile', '')
        }
        
        # Create files
        created_files = []
        for path, content in self._flatten_structure(structure, dev_path):
            path.parent.mkdir(parents=True, exist_ok=True)
            
            if isinstance(content, dict):
                path.mkdir(exist_ok=True)
            else:
                with open(path, 'w') as f:
                    f.write(content)
                created_files.append(str(path.relative_to(dev_path)))
        
        # Initialize git repository
        repo = git.Repo.init(dev_path)
        repo.index.add(created_files)
        repo.index.commit("Initial commit - LlamaSearchAI project template")
        
        # Create GitHub repository
        try:
            github_repo = self.organization.create_repo(
                name=project_name,
                description=f"LlamaSearchAI {project_name} project",
                private=True,
                auto_init=False
            )
            
            # Add remote
            origin = repo.create_remote('origin', github_repo.clone_url)
            
            result = {
                'success': True,
                'local_path': str(dev_path),
                'github_url': github_repo.html_url,
                'files_created': len(created_files),
                'structure': structure
            }
            
        except Exception as e:
            result = {
                'success': True,
                'local_path': str(dev_path),
                'github_url': None,
                'error': f"Local project created but GitHub repo creation failed: {e}",
                'files_created': len(created_files)
            }
        
        return result
    
    def _flatten_structure(self, structure: Dict, base_path: Path, current_path: Path = None):
        """Flatten nested structure for file creation"""
        if current_path is None:
            current_path = base_path
        
        for name, content in structure.items():
            path = current_path / name
            if isinstance(content, dict):
                yield from self._flatten_structure(content, base_path, path)
            else:
                yield path, content
    
    async def _analyze_project_templates(self) -> Dict[str, str]:
        """Analyze existing projects to create templates"""
        templates = {}
        
        # Analyze existing repos for common patterns
        for repo_name in ['llamaagent', 'llamagraph', 'llama-cli']:
            if repo_name in self.repo_metadata:
                local_path = self.local_repos_path / self.org_name / repo_name
                
                if local_path.exists():
                    # Extract common file templates
                    template_files = {
                        'setup.py': 'setup.py',
                        'requirements': 'requirements.txt',
                        'gitignore': '.gitignore',
                        'dockerfile': 'Dockerfile',
                        'github_workflow': '.github/workflows/test.yml'
                    }
                    
                    for template_name, file_name in template_files.items():
                        file_path = local_path / file_name
                        if file_path.exists():
                            try:
                                with open(file_path, 'r') as f:
                                    templates[template_name] = f.read()
                            except:
                                pass
        
        # Add default templates if not found
        if 'requirements' not in templates:
            templates['requirements'] = """# LlamaSearchAI Project Requirements
fastapi>=0.104.0
pydantic>=2.0.0
aiohttp>=3.9.0
python-dotenv>=1.0.0
pytest>=7.4.0
pytest-asyncio>=0.21.0
"""
        
        if 'gitignore' not in templates:
            templates['gitignore'] = """# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
env/
venv/
ENV/
.env

# IDEs
.vscode/
.idea/
*.swp
*.swo

# Testing
.pytest_cache/
.coverage
htmlcov/

# Distribution
dist/
build/
*.egg-info/
"""
        
        return templates
    
    async def _save_repo_metadata(self, repos: List[Dict]):
        """Save repository metadata to file"""
        metadata_file = self.local_repos_path / f"{self.org_name}_metadata.json"
        metadata_file.parent.mkdir(parents=True, exist_ok=True)
        
        async with aiofiles.open(metadata_file, 'w') as f:
            await f.write(json.dumps(repos, indent=2))
        
        logger.info(f"Saved metadata for {len(repos)} repositories")
    
    async def generate_documentation(self) -> Path:
        """Generate comprehensive documentation for all repositories"""
        doc_dir = self.local_repos_path / "documentation" / datetime.now().strftime("%Y%m%d_%H%M%S")
        doc_dir.mkdir(parents=True, exist_ok=True)
        
        # Generate main documentation
        main_doc = doc_dir / "LlamaSearchAI_Documentation.md"
        
        doc_content = f"""# LlamaSearchAI Complete Documentation

Generated: {datetime.now().isoformat()}

## Organization Overview

LlamaSearchAI is an advanced AI-powered search and agent ecosystem consisting of {len(self.repo_metadata)} repositories.

## Repository Documentation

"""
        
        # Generate documentation for each repo
        for repo_name, repo_info in self.repo_metadata.items():
            doc_content += f"""
### {repo_name}

**Description**: {repo_info.get('description', 'No description available')}

**Details**:
- Language: {repo_info.get('language', 'Unknown')}
- Stars: {repo_info.get('stars', 0)}
- Created: {repo_info.get('created_at', 'Unknown')}
- Topics: {', '.join(repo_info.get('topics', []))}

"""
            
            # Add README content if available
            local_path = self.local_repos_path / self.org_name / repo_name / 'README.md'
            if local_path.exists():
                try:
                    with open(local_path, 'r') as f:
                        readme = f.read()
                        # Add first section of README
                        doc_content += "**README Extract**:\n```\n"
                        doc_content += readme[:1000] + "...\n```\n\n"
                except:
                    pass
        
        # Add architecture overview
        ecosystem_analysis = await self.analyze_ecosystem()
        
        doc_content += f"""
## Ecosystem Analysis

### Language Distribution
"""
        for lang, count in ecosystem_analysis['languages'].items():
            doc_content += f"- {lang}: {count} repositories\n"
        
        doc_content += f"""
### Common Topics
"""
        for topic, count in sorted(ecosystem_analysis['topics'].items(), key=lambda x: x[1], reverse=True)[:10]:
            doc_content += f"- {topic}: {count} repositories\n"
        
        # Save documentation
        async with aiofiles.open(main_doc, 'w') as f:
            await f.write(doc_content)
        
        logger.info(f"Generated documentation at: {main_doc}")
        
        return doc_dir


# CLI Extension for LlamaSearchAI
async def llamasearch_cli():
    """Enhanced CLI for LlamaSearchAI management"""
    import click
    
    @click.group()
    @click.pass_context
    def cli(ctx):
        """LlamaSearchAI Repository Manager"""
        ctx.ensure_object(dict)
    
    @cli.command()
    @click.pass_context
    async def scan(ctx):
        """Scan all LlamaSearchAI repositories"""
        manager = LlamaSearchManager()
        await manager.initialize()
        
        repos = await manager.scan_organization_repos()
        click.echo(f"Found {len(repos)} repositories:")
        
        for repo in repos:
            status = "🔒" if repo['private'] else "🌐"
            click.echo(f"{status} {repo['name']} - ⭐ {repo['stars']} - {repo['language'] or 'No language'}")
        
        await manager.cleanup()
    
    @cli.command()
    @click.option('--output', '-o', help='Output directory')
    @click.pass_context
    async def generate(ctx, output):
        """Generate concatenated files for all repositories"""
        manager = LlamaSearchManager()
        await manager.initialize()
        
        click.echo("Cloning/updating repositories...")
        await manager.clone_all_repos()
        
        click.echo("Generating concatenated files...")
        results = await manager.generate_concatenated_files(output)
        
        click.echo(f"\n✅ Generated {len(results)} concatenated files")
        
        await manager.cleanup()
    
    @cli.command()
    @click.pass_context
    async def analyze(ctx):
        """Analyze the LlamaSearchAI ecosystem"""
        manager = LlamaSearchManager()
        await manager.initialize()
        
        analysis = await manager.analyze_ecosystem()
        
        click.echo("\n🔍 LlamaSearchAI Ecosystem Analysis")
        click.echo(f"Total Repositories: {analysis['total_repos']}")
        click.echo("\nLanguages:")
        for lang, count in analysis['languages'].items():
            click.echo(f"  - {lang}: {count}")
        
        if 'architecture' in analysis and 'ai_analysis' in analysis['architecture']:
            click.echo("\n🤖 AI Architecture Analysis:")
            click.echo(analysis['architecture']['ai_analysis'])
        
        await manager.cleanup()
    
    @cli.command()
    @click.argument('project_name')
    @click.pass_context
    async def create(ctx, project_name):
        """Create a new LlamaSearchAI project"""
        manager = LlamaSearchManager()
        await manager.initialize()
        
        click.echo(f"Creating project: {project_name}")
        result = await manager.create_development_environment(project_name)
        
        if result['success']:
            click.echo(f"✅ Project created at: {result['local_path']}")
            if result.get('github_url'):
                click.echo(f"📦 GitHub repository: {result['github_url']}")
        else:
            click.echo(f"❌ Failed: {result.get('error')}")
        
        await manager.cleanup()
    
    return cli


if __name__ == "__main__":
    # Run the CLI
    cli = asyncio.run(llamasearch_cli())
    cli()