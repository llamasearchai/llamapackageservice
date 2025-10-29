"""
Web Dashboard for GitHub Repository Manager
"""
from fastapi import FastAPI, WebSocket, HTTPException, BackgroundTasks
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from fastapi.responses import HTMLResponse, JSONResponse
from pydantic import BaseModel
from typing import Dict, List, Optional, Any
import json
import asyncio
from datetime import datetime
from pathlib import Path

from ..core.repo_manager import GitHubRepoManager
from ..utils.logger import setup_logger

logger = setup_logger(__name__)


class RepoAnalysisRequest(BaseModel):
    repo_path: str
    detailed: bool = False


class CommandRequest(BaseModel):
    command: str
    repo: Optional[str] = None


class BatchOperationRequest(BaseModel):
    operation: str
    repositories: List[str]


class CreateRepoRequest(BaseModel):
    name: str
    description: str = ""
    private: bool = False


def create_app(config_path: str = "config/github_config.yaml"):
    app = FastAPI(title="GitHub Repository Manager")
    
    # Initialize manager
    manager = GitHubRepoManager(config_path)
    
    # Setup templates
    templates = Jinja2Templates(directory="src/web/templates")
    
    # Mount static files
    app.mount("/static", StaticFiles(directory="src/web/static"), name="static")
    
    # Store WebSocket connections
    connections = []
    
    @app.on_event("startup")
    async def startup_event():
        await manager.initialize()
        logger.info("Web dashboard started")
    
    @app.on_event("shutdown")
    async def shutdown_event():
        await manager.cleanup()
    
    @app.get("/", response_class=HTMLResponse)
    async def dashboard():
        repos = manager.scan_local_repositories()
        return templates.TemplateResponse("dashboard.html", {
            "request": {},
            "repositories": repos,
            "total_repos": len(repos),
            "languages": get_language_stats(repos)
        })
    
    @app.get("/api/repositories")
    async def list_repositories():
        repos = manager.scan_local_repositories()
        return JSONResponse(content=repos)
    
    @app.get("/api/repository/{repo_name}")
    async def get_repository(repo_name: str):
        repo_info = manager.repo_cache.get(repo_name)
        if not repo_info:
            raise HTTPException(status_code=404, detail="Repository not found")
        return JSONResponse(content=repo_info)
    
    @app.post("/api/repository/{repo_name}/analyze")
    async def analyze_repository(repo_name: str, request: RepoAnalysisRequest):
        try:
            result = await manager.ai_analyze_repository(request.repo_path)
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.post("/api/repository/{repo_name}/sync")
    async def sync_repository(repo_name: str):
        try:
            result = await manager.sync_repository(repo_name)
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.get("/api/repository/{repo_name}/issues")
    async def get_issues(repo_name: str):
        try:
            result = await manager.analyze_issues(repo_name)
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.post("/api/repositories")
    async def create_repository(request: CreateRepoRequest):
        try:
            result = await manager.create_repository(
                request.name,
                request.description,
                request.private
            )
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.post("/api/batch")
    async def batch_operation(request: BatchOperationRequest, background_tasks: BackgroundTasks):
        try:
            # Run batch operation in background
            task_id = f"batch_{datetime.now().timestamp()}"
            
            async def run_batch():
                result = await manager.batch_operation(
                    request.operation,
                    request.repositories
                )
                # Store result or send via WebSocket
                await notify_clients({
                    'type': 'batch_complete',
                    'task_id': task_id,
                    'result': result
                })
            
            background_tasks.add_task(run_batch)
            
            return JSONResponse(content={
                'task_id': task_id,
                'status': 'started'
            })
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.post("/api/command")
    async def execute_command(request: CommandRequest):
        try:
            result = await manager.ollama.analyze_code_with_tools(
                request.repo or ".",
                request.command
            )
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.get("/api/search")
    async def search_code(query: str, repo: Optional[str] = None, language: Optional[str] = None):
        try:
            results = await manager.mcp_client.search_code(query, repo, language)
            return JSONResponse(content={'results': results})
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.websocket("/ws")
    async def websocket_endpoint(websocket: WebSocket):
        await websocket.accept()
        connections.append(websocket)
        
        try:
            while True:
                data = await websocket.receive_text()
                request = json.loads(data)
                
                if request['type'] == 'analyze_repo':
                    result = await manager.ai_analyze_repository(request['repo_path'])
                    await websocket.send_json({
                        'type': 'analysis_result',
                        'data': result
                    })
                
                elif request['type'] == 'monitor_repo':
                    # Start monitoring
                    repo_name = request['repo_name']
                    
                    async def on_change(event_type, data):
                        await websocket.send_json({
                            'type': 'repo_event',
                            'event': event_type,
                            'data': data
                        })
                    
                    # This would connect to the MCP monitor
                    await manager.mcp_client.monitor_repository(
                        repo_name,
                        request.get('events', ['commit', 'issue', 'pr'])
                    )
                
                elif request['type'] == 'execute_command':
                    result = await manager.ollama.analyze_code_with_tools(
                        request.get('repo', '.'),
                        request['command']
                    )
                    await websocket.send_json({
                        'type': 'command_result',
                        'data': result
                    })
                
        except Exception as e:
            logger.error(f"WebSocket error: {e}")
        finally:
            connections.remove(websocket)
    
    async def notify_clients(message: Dict):
        """Notify all connected WebSocket clients"""
        for connection in connections:
            try:
                await connection.send_json(message)
            except:
                pass
    
    def get_language_stats(repos: List[Dict]) -> Dict[str, int]:
        """Calculate language statistics across repositories"""
        stats = {}
        for repo in repos:
            # This would be populated from actual repo analysis
            pass
        return stats
    
    @app.get("/api/stats")
    async def get_statistics():
        repos = manager.scan_local_repositories()
        
        stats = {
            'total_repositories': len(repos),
            'languages': {},
            'total_commits': 0,
            'active_branches': 0,
            'modified_repos': 0
        }
        
        for repo in repos:
            if repo['status'].get('is_dirty'):
                stats['modified_repos'] += 1
        
        return JSONResponse(content=stats)
    
    @app.get("/api/notifications")
    async def get_notifications():
        try:
            result = await manager.mcp_client.get_notifications()
            return JSONResponse(content=result)
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.get("/api/mcp/status")
    async def get_mcp_status():
        status = {}
        for server_name in ['github', 'filesystem', 'project']:
            status[server_name] = await manager.mcp_client.get_server_status(server_name)
        return JSONResponse(content=status)
    
    return app