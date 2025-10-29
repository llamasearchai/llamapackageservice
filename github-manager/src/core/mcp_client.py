"""
MCP Client for GitHub Operations
"""
import asyncio
import json
import logging
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
import websockets
import aiohttp

logger = logging.getLogger(__name__)


@dataclass
class MCPServer:
    name: str
    host: str
    port: int
    capabilities: List[str]
    connection: Optional[websockets.WebSocketClientProtocol] = None


class MCPClient:
    def __init__(self):
        self.servers: Dict[str, MCPServer] = {}
        self.active_connections = {}
        
    async def connect_servers(self, server_configs: List[Dict]):
        """Connect to MCP servers"""
        for config in server_configs:
            server = MCPServer(
                name=config['name'],
                host=config.get('host', 'localhost'),
                port=config['port'],
                capabilities=config.get('capabilities', [])
            )
            
            try:
                await self.connect_server(server)
                self.servers[server.name] = server
                logger.info(f"Connected to MCP server: {server.name}")
            except Exception as e:
                logger.error(f"Failed to connect to {server.name}: {e}")
    
    async def connect_server(self, server: MCPServer):
        """Connect to a single MCP server"""
        uri = f"ws://{server.host}:{server.port}"
        server.connection = await websockets.connect(uri)
        
        # Send initialization
        await server.connection.send(json.dumps({
            'type': 'initialize',
            'client': 'github-repo-manager',
            'version': '1.0.0'
        }))
        
        # Wait for acknowledgment
        response = await server.connection.recv()
        data = json.loads(response)
        
        if data.get('type') == 'initialized':
            server.capabilities = data.get('capabilities', [])
    
    async def call_tool(self, server_name: str, tool_name: str, params: Dict[str, Any]) -> Dict:
        """Call a tool on an MCP server"""
        server = self.servers.get(server_name)
        if not server or not server.connection:
            raise ValueError(f"Server {server_name} not connected")
        
        # Send tool request
        request = {
            'type': 'tool_call',
            'tool': tool_name,
            'params': params,
            'id': f"{server_name}_{tool_name}_{asyncio.get_event_loop().time()}"
        }
        
        await server.connection.send(json.dumps(request))
        
        # Wait for response
        response = await server.connection.recv()
        return json.loads(response)
    
    async def github_operation(self, operation: str, params: Dict[str, Any]) -> Dict:
        """Execute GitHub operation via MCP"""
        return await self.call_tool('github', operation, params)
    
    async def list_repositories(self, username: Optional[str] = None) -> List[Dict]:
        """List repositories via MCP"""
        result = await self.github_operation('list_repositories', {
            'username': username
        })
        return result.get('repositories', [])
    
    async def create_repository(self, name: str, description: str = "", private: bool = False) -> Dict:
        """Create repository via MCP"""
        return await self.github_operation('create_repository', {
            'name': name,
            'description': description,
            'private': private
        })
    
    async def analyze_repository_issues(self, repo_name: str) -> Dict:
        """Analyze repository issues via MCP"""
        return await self.github_operation('analyze_repository_issues', {
            'repo_name': repo_name
        })
    
    async def create_pull_request(self, repo_name: str, title: str, body: str, 
                                 head: str, base: str = "main") -> Dict:
        """Create pull request via MCP"""
        return await self.github_operation('create_pull_request', {
            'repo_name': repo_name,
            'title': title,
            'body': body,
            'head': head,
            'base': base
        })
    
    async def review_pull_request(self, repo_name: str, pr_number: int) -> Dict:
        """Review pull request via MCP"""
        return await self.github_operation('review_pull_request', {
            'repo_name': repo_name,
            'pr_number': pr_number
        })
    
    async def execute_workflow(self, repo_name: str, workflow_id: str, 
                             ref: str = "main", inputs: Dict = None) -> Dict:
        """Execute GitHub Actions workflow via MCP"""
        return await self.github_operation('execute_workflow', {
            'repo_name': repo_name,
            'workflow_id': workflow_id,
            'ref': ref,
            'inputs': inputs or {}
        })
    
    async def get_repository_insights(self, repo_name: str) -> Dict:
        """Get repository insights via MCP"""
        return await self.github_operation('get_repository_insights', {
            'repo_name': repo_name
        })
    
    async def search_code(self, query: str, repo: Optional[str] = None, 
                         language: Optional[str] = None) -> List[Dict]:
        """Search code via MCP"""
        params = {'query': query}
        if repo:
            params['repo'] = repo
        if language:
            params['language'] = language
            
        result = await self.github_operation('search_code', params)
        return result.get('results', [])
    
    async def get_file_content(self, repo_name: str, path: str, ref: str = "main") -> str:
        """Get file content via MCP"""
        result = await self.github_operation('get_file_content', {
            'repo_name': repo_name,
            'path': path,
            'ref': ref
        })
        return result.get('content', '')
    
    async def update_file(self, repo_name: str, path: str, content: str, 
                         message: str, branch: str = "main") -> Dict:
        """Update file via MCP"""
        return await self.github_operation('update_file', {
            'repo_name': repo_name,
            'path': path,
            'content': content,
            'message': message,
            'branch': branch
        })
    
    async def manage_secrets(self, repo_name: str, action: str, 
                           secret_name: str, value: Optional[str] = None) -> Dict:
        """Manage repository secrets via MCP"""
        params = {
            'repo_name': repo_name,
            'action': action,
            'secret_name': secret_name
        }
        if value:
            params['value'] = value
            
        return await self.github_operation('manage_secrets', params)
    
    async def get_notifications(self, all: bool = False, participating: bool = False) -> List[Dict]:
        """Get GitHub notifications via MCP"""
        result = await self.github_operation('get_notifications', {
            'all': all,
            'participating': participating
        })
        return result.get('notifications', [])
    
    async def monitor_repository(self, repo_name: str, events: List[str]) -> Dict:
        """Start monitoring repository for events via MCP"""
        return await self.github_operation('monitor_repository', {
            'repo_name': repo_name,
            'events': events
        })
    
    async def stop_monitoring(self, repo_name: str) -> Dict:
        """Stop monitoring repository via MCP"""
        return await self.github_operation('stop_monitoring', {
            'repo_name': repo_name
        })
    
    async def get_server_status(self, server_name: str) -> Dict:
        """Get MCP server status"""
        server = self.servers.get(server_name)
        if not server:
            return {'status': 'not_found'}
        
        return {
            'status': 'connected' if server.connection else 'disconnected',
            'capabilities': server.capabilities,
            'host': server.host,
            'port': server.port
        }
    
    async def broadcast_operation(self, operation: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Broadcast operation to all capable servers"""
        results = {}
        
        for server_name, server in self.servers.items():
            if operation in server.capabilities:
                try:
                    results[server_name] = await self.call_tool(server_name, operation, params)
                except Exception as e:
                    results[server_name] = {'error': str(e)}
        
        return results
    
    async def disconnect(self):
        """Disconnect from all MCP servers"""
        for server in self.servers.values():
            if server.connection:
                await server.connection.close()
        self.servers.clear()