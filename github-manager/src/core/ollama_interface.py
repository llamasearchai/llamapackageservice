"""
Ollama Interface with Function Calling Capabilities
"""
import asyncio
import json
import subprocess
from typing import Dict, List, Any, Optional, Callable
from pathlib import Path
import logging
import base64
import aiohttp
from dataclasses import dataclass

logger = logging.getLogger(__name__)


@dataclass
class Tool:
    name: str
    description: str
    parameters: Dict[str, Any]
    function: Optional[Callable] = None


class OllamaInterface:
    def __init__(self, host: str = "http://localhost:11434"):
        self.host = host
        self.session = None
        self.models = {
            'code': 'llama3.1:8b',
            'vision': 'llava:13b',
            'function_calling': 'llama3.1:8b'
        }
        self.tools = {}
        self._setup_tools()
        
    async def initialize_models(self):
        """Pull required models if not available"""
        self.session = aiohttp.ClientSession()
        
        for model_type, model_name in self.models.items():
            if not await self.model_exists(model_name):
                logger.info(f"Pulling {model_name}...")
                await self.pull_model(model_name)
            else:
                logger.info(f"Model {model_name} already available")
    
    async def model_exists(self, model_name: str) -> bool:
        """Check if model exists"""
        try:
            async with self.session.get(f"{self.host}/api/tags") as response:
                if response.status == 200:
                    data = await response.json()
                    models = [m['name'] for m in data.get('models', [])]
                    return model_name in models
        except Exception as e:
            logger.error(f"Error checking model: {e}")
        return False
    
    async def pull_model(self, model_name: str):
        """Pull a model from Ollama"""
        async with self.session.post(
            f"{self.host}/api/pull",
            json={"name": model_name}
        ) as response:
            async for line in response.content:
                if line:
                    data = json.loads(line)
                    if 'status' in data:
                        logger.info(f"Pull status: {data['status']}")
    
    def _setup_tools(self):
        """Setup available tools for function calling"""
        self.tools = {
            'read_file': Tool(
                name='read_file',
                description='Read contents of a file',
                parameters={
                    'type': 'object',
                    'properties': {
                        'file_path': {
                            'type': 'string',
                            'description': 'Path to the file to read'
                        }
                    },
                    'required': ['file_path']
                },
                function=self._read_file
            ),
            'execute_command': Tool(
                name='execute_command',
                description='Execute a shell command',
                parameters={
                    'type': 'object',
                    'properties': {
                        'command': {
                            'type': 'string',
                            'description': 'Shell command to execute'
                        },
                        'cwd': {
                            'type': 'string',
                            'description': 'Working directory (optional)'
                        }
                    },
                    'required': ['command']
                },
                function=self._execute_command
            ),
            'git_status': Tool(
                name='git_status',
                description='Get git status of repository',
                parameters={
                    'type': 'object',
                    'properties': {
                        'repo_path': {
                            'type': 'string',
                            'description': 'Path to git repository'
                        }
                    },
                    'required': ['repo_path']
                },
                function=self._git_status
            ),
            'search_code': Tool(
                name='search_code',
                description='Search for code patterns in files',
                parameters={
                    'type': 'object',
                    'properties': {
                        'pattern': {
                            'type': 'string',
                            'description': 'Search pattern (regex)'
                        },
                        'path': {
                            'type': 'string',
                            'description': 'Directory to search in'
                        },
                        'file_pattern': {
                            'type': 'string',
                            'description': 'File pattern to match (e.g., *.py)'
                        }
                    },
                    'required': ['pattern', 'path']
                },
                function=self._search_code
            ),
            'analyze_dependencies': Tool(
                name='analyze_dependencies',
                description='Analyze project dependencies',
                parameters={
                    'type': 'object',
                    'properties': {
                        'project_path': {
                            'type': 'string',
                            'description': 'Path to project'
                        }
                    },
                    'required': ['project_path']
                },
                function=self._analyze_dependencies
            )
        }
    
    async def analyze_code_with_tools(self, repo_path: str, query: str) -> Dict:
        """Analyze code using function calling capabilities"""
        
        # Convert tools to Ollama format
        tools_list = [
            {
                'type': 'function',
                'function': {
                    'name': tool.name,
                    'description': tool.description,
                    'parameters': tool.parameters
                }
            }
            for tool in self.tools.values()
        ]
        
        messages = [
            {
                'role': 'system',
                'content': 'You are a code analysis assistant with access to various tools. Use them to analyze the repository and answer questions.'
            },
            {
                'role': 'user',
                'content': f"Repository path: {repo_path}\n\nQuery: {query}"
            }
        ]
        
        # Make initial request with tools
        async with self.session.post(
            f"{self.host}/api/chat",
            json={
                'model': self.models['function_calling'],
                'messages': messages,
                'tools': tools_list,
                'stream': False
            }
        ) as response:
            result = await response.json()
        
        # Handle tool calls
        if 'message' in result and 'tool_calls' in result['message']:
            tool_results = []
            
            for tool_call in result['message']['tool_calls']:
                function_name = tool_call['function']['name']
                arguments = json.loads(tool_call['function']['arguments'])
                
                # Execute tool function
                tool_result = await self.execute_tool_function(function_name, arguments)
                
                tool_results.append({
                    'role': 'tool',
                    'content': json.dumps(tool_result)
                })
            
            # Add tool results to messages
            messages.append(result['message'])
            messages.extend(tool_results)
            
            # Get final response
            async with self.session.post(
                f"{self.host}/api/chat",
                json={
                    'model': self.models['function_calling'],
                    'messages': messages,
                    'stream': False
                }
            ) as response:
                final_result = await response.json()
            
            return {
                'analysis': final_result['message']['content'],
                'tool_calls_made': len(tool_results),
                'tools_used': [tc['function']['name'] for tc in result['message']['tool_calls']]
            }
        
        return {
            'analysis': result['message']['content'],
            'tool_calls_made': 0,
            'tools_used': []
        }
    
    async def analyze_with_vision(self, repo_path: str, prompt: str, repo_data: Dict) -> Dict:
        """Analyze repository using vision model for diagrams/screenshots"""
        # Find images in repository
        image_files = []
        path = Path(repo_path)
        
        for ext in ['.png', '.jpg', '.jpeg', '.svg']:
            image_files.extend(path.rglob(f'*{ext}'))
        
        if not image_files:
            return await self.analyze_code(repo_path, prompt, repo_data)
        
        # Analyze first few images
        analyses = []
        for img_file in image_files[:3]:  # Limit to 3 images
            try:
                with open(img_file, 'rb') as f:
                    image_data = base64.b64encode(f.read()).decode()
                
                async with self.session.post(
                    f"{self.host}/api/generate",
                    json={
                        'model': self.models['vision'],
                        'prompt': f"Analyze this image from the repository: {prompt}",
                        'images': [image_data],
                        'stream': False
                    }
                ) as response:
                    result = await response.json()
                    analyses.append({
                        'file': str(img_file.relative_to(path)),
                        'analysis': result['response']
                    })
            except Exception as e:
                logger.error(f"Error analyzing image {img_file}: {e}")
        
        # Combine with code analysis
        code_analysis = await self.analyze_code(repo_path, prompt, repo_data)
        
        return {
            'code_analysis': code_analysis,
            'visual_analyses': analyses,
            'has_visuals': True
        }
    
    async def analyze_code(self, repo_path: str, prompt: str, repo_data: Dict) -> Dict:
        """Analyze code without vision capabilities"""
        context = f"""
        Repository: {repo_path}
        Files: {len(repo_data['files'])} files
        Languages: {json.dumps(repo_data['languages'], indent=2)}
        Dependencies: {json.dumps(repo_data['dependencies'], indent=2)}
        Tests: {len(repo_data['tests'])} test files
        Documentation: {len(repo_data['docs'])} documentation files
        
        {prompt}
        """
        
        async with self.session.post(
            f"{self.host}/api/generate",
            json={
                'model': self.models['code'],
                'prompt': context,
                'stream': False
            }
        ) as response:
            result = await response.json()
        
        return {
            'analysis': result['response'],
            'repo_stats': {
                'files': len(repo_data['files']),
                'languages': repo_data['languages'],
                'has_tests': len(repo_data['tests']) > 0,
                'has_docs': len(repo_data['docs']) > 0
            }
        }
    
    async def analyze_text(self, prompt: str) -> str:
        """Simple text analysis"""
        async with self.session.post(
            f"{self.host}/api/generate",
            json={
                'model': self.models['code'],
                'prompt': prompt,
                'stream': False
            }
        ) as response:
            result = await response.json()
            return result['response']
    
    async def execute_tool_function(self, function_name: str, arguments: Dict) -> Any:
        """Execute tool functions"""
        tool = self.tools.get(function_name)
        if not tool or not tool.function:
            return {'error': f'Unknown tool: {function_name}'}
        
        try:
            return await tool.function(**arguments)
        except Exception as e:
            return {'error': str(e)}
    
    # Tool implementations
    async def _read_file(self, file_path: str) -> Dict:
        """Read file contents"""
        try:
            with open(file_path, 'r') as f:
                content = f.read()
            return {'content': content, 'success': True}
        except Exception as e:
            return {'error': str(e), 'success': False}
    
    async def _execute_command(self, command: str, cwd: Optional[str] = None) -> Dict:
        """Execute shell command"""
        try:
            result = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=30,
                cwd=cwd
            )
            return {
                'stdout': result.stdout,
                'stderr': result.stderr,
                'returncode': result.returncode,
                'success': result.returncode == 0
            }
        except Exception as e:
            return {'error': str(e), 'success': False}
    
    async def _git_status(self, repo_path: str) -> Dict:
        """Get git status"""
        try:
            import git
            repo = git.Repo(repo_path)
            return {
                'branch': repo.active_branch.name,
                'is_dirty': repo.is_dirty(),
                'modified_files': [item.a_path for item in repo.index.diff(None)],
                'untracked_files': repo.untracked_files,
                'success': True
            }
        except Exception as e:
            return {'error': str(e), 'success': False}
    
    async def _search_code(self, pattern: str, path: str, file_pattern: str = '*') -> Dict:
        """Search for code patterns"""
        try:
            import re
            matches = []
            path_obj = Path(path)
            
            for file_path in path_obj.rglob(file_pattern):
                if file_path.is_file():
                    try:
                        with open(file_path, 'r') as f:
                            content = f.read()
                            for i, line in enumerate(content.splitlines(), 1):
                                if re.search(pattern, line):
                                    matches.append({
                                        'file': str(file_path.relative_to(path_obj)),
                                        'line': i,
                                        'content': line.strip()
                                    })
                    except:
                        pass
            
            return {'matches': matches[:50], 'total': len(matches), 'success': True}
        except Exception as e:
            return {'error': str(e), 'success': False}
    
    async def _analyze_dependencies(self, project_path: str) -> Dict:
        """Analyze project dependencies"""
        try:
            deps = {}
            path = Path(project_path)
            
            # Check for various dependency files
            if (path / 'package.json').exists():
                with open(path / 'package.json') as f:
                    data = json.load(f)
                    deps['npm'] = {
                        'dependencies': list(data.get('dependencies', {}).keys()),
                        'devDependencies': list(data.get('devDependencies', {}).keys())
                    }
            
            if (path / 'requirements.txt').exists():
                with open(path / 'requirements.txt') as f:
                    deps['pip'] = [line.strip().split('==')[0] for line in f if line.strip()]
            
            if (path / 'Cargo.toml').exists():
                deps['cargo'] = {'found': True}  # Simplified
            
            return {'dependencies': deps, 'success': True}
        except Exception as e:
            return {'error': str(e), 'success': False}
    
    async def generate_code(self, prompt: str, language: str = "python") -> str:
        """Generate code based on prompt"""
        code_prompt = f"Generate {language} code for: {prompt}\n\nProvide only the code without explanations."
        
        async with self.session.post(
            f"{self.host}/api/generate",
            json={
                'model': self.models['code'],
                'prompt': code_prompt,
                'stream': False
            }
        ) as response:
            result = await response.json()
            return result['response']
    
    async def cleanup(self):
        """Cleanup resources"""
        if self.session:
            await self.session.close()