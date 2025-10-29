"""FastAPI server for web dashboard and API."""
import asyncio
import base64
import json
from datetime import datetime
from typing import Any, Dict, List, Optional
import logging

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field
import uvicorn

from ..interfaces import Action, ActionType, OperationStatus
from ..core.orchestrator import OperationOrchestrator, Operation
from ..integrations.github_manager import GitHubManager
from ..security.guardian import SecurityGuardian
from ..state.manager import StateManager, StorageBackend
from ..core.performance import PerformanceOptimizer


logger = logging.getLogger(__name__)


# API Models
class ActionRequest(BaseModel):
    """Request model for actions."""
    type: str
    target: Optional[str] = None
    value: Optional[Any] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class OperationRequest(BaseModel):
    """Request model for operations."""
    objective: str
    actions: Optional[List[ActionRequest]] = None
    max_iterations: int = Field(default=10, ge=1, le=50)
    continue_on_failure: bool = False


class SystemStatus(BaseModel):
    """System status response."""
    status: str
    uptime: float
    active_operations: int
    performance_metrics: Dict[str, Any]
    security_mode: str


class GitHubRequest(BaseModel):
    """GitHub operation request."""
    operation: str
    repo_name: str
    parameters: Dict[str, Any] = Field(default_factory=dict)


# WebSocket connection manager
class ConnectionManager:
    """Manages WebSocket connections."""
    
    def __init__(self):
        self.active_connections: List[WebSocket] = []
        
    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.append(websocket)
        
    def disconnect(self, websocket: WebSocket):
        self.active_connections.remove(websocket)
        
    async def send_personal_message(self, message: str, websocket: WebSocket):
        await websocket.send_text(message)
        
    async def broadcast(self, message: str):
        for connection in self.active_connections:
            try:
                await connection.send_text(message)
            except:
                pass


# Create FastAPI app
app = FastAPI(
    title="Operate Enhanced API",
    description="API for the enhanced self-operating computer framework",
    version="2.0.0"
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global instances
manager = ConnectionManager()
orchestrator: Optional[OperationOrchestrator] = None
github_manager: Optional[GitHubManager] = None
security_guardian: Optional[SecurityGuardian] = None
state_manager: Optional[StateManager] = None
performance_optimizer: Optional[PerformanceOptimizer] = None
start_time = datetime.utcnow()


@app.on_event("startup")
async def startup_event():
    """Initialize services on startup."""
    global orchestrator, github_manager, security_guardian, state_manager, performance_optimizer
    
    # Initialize components
    security_guardian = SecurityGuardian()
    state_manager = StateManager(StorageBackend(type="file"))
    performance_optimizer = PerformanceOptimizer()
    
    # Initialize orchestrator
    from ..utils.action_handler import ActionHandler
    action_handler = ActionHandler()
    
    orchestrator = OperationOrchestrator(
        action_interface=action_handler,
        state_manager=state_manager,
        security_validator=security_guardian,
        enable_learning=True
    )
    
    logger.info("API server initialized")


@app.get("/")
async def root():
    """Root endpoint."""
    return {"message": "Operate Enhanced API", "version": "2.0.0"}


@app.get("/status", response_model=SystemStatus)
async def get_status():
    """Get system status."""
    uptime = (datetime.utcnow() - start_time).total_seconds()
    
    # Get performance metrics
    perf_metrics = await performance_optimizer.get_metrics() if performance_optimizer else {}
    
    # Get security mode
    is_sandbox = await security_guardian.is_sandbox_mode() if security_guardian else True
    
    return SystemStatus(
        status="operational",
        uptime=uptime,
        active_operations=len(orchestrator.executing) if orchestrator else 0,
        performance_metrics=perf_metrics,
        security_mode="sandbox" if is_sandbox else "production"
    )


@app.post("/execute")
async def execute_operation(request: OperationRequest):
    """Execute an operation."""
    try:
        # Create actions from request
        actions = []
        if request.actions:
            for action_req in request.actions:
                action = Action(
                    id=f"api_{datetime.utcnow().timestamp()}",
                    type=ActionType(action_req.type),
                    target=action_req.target,
                    value=action_req.value,
                    metadata=action_req.metadata
                )
                actions.append(action)
        
        # Create operation
        operation = Operation(
            action=actions[0] if actions else None,
            metadata={
                "objective": request.objective,
                "max_iterations": request.max_iterations,
                "continue_on_failure": request.continue_on_failure
            }
        )
        
        # Execute
        result = await orchestrator.execute_operation(operation)
        
        # Broadcast status
        await manager.broadcast(json.dumps({
            "type": "operation_complete",
            "operation_id": operation.id,
            "status": result.status.value,
            "timestamp": result.timestamp.isoformat()
        }))
        
        return {
            "operation_id": operation.id,
            "status": result.status.value,
            "result": result.result,
            "error": str(result.error) if result.error else None
        }
        
    except Exception as e:
        logger.error(f"Operation execution failed: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/github")
async def github_operation(request: GitHubRequest):
    """Execute GitHub operation."""
    if not github_manager:
        # Initialize GitHub manager
        token = request.parameters.get("token")
        if not token:
            raise HTTPException(status_code=400, detail="GitHub token required")
            
        global github_manager
        github_manager = GitHubManager(token)
        await github_manager.initialize()
    
    try:
        if request.operation == "analyze":
            result = await github_manager.analyze_codebase(request.parameters["path"])
        elif request.operation == "create_pr":
            result = await github_manager.create_pull_request(
                request.repo_name,
                request.parameters["title"],
                request.parameters["body"],
                request.parameters["head"],
                request.parameters.get("base", "main")
            )
        elif request.operation == "review_pr":
            result = await github_manager.review_pull_request(
                request.repo_name,
                request.parameters["pr_number"]
            )
        else:
            raise HTTPException(status_code=400, detail=f"Unknown operation: {request.operation}")
            
        return {"success": True, "result": result}
        
    except Exception as e:
        logger.error(f"GitHub operation failed: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/operations/history")
async def get_operation_history(limit: int = 100):
    """Get operation history."""
    if not state_manager:
        return []
        
    history = await state_manager.get_operation_history(limit)
    return history


@app.get("/patterns")
async def get_patterns():
    """Get saved patterns."""
    if not state_manager:
        return []
        
    pattern_keys = await state_manager._store.keys("patterns/*")
    patterns = []
    
    for key in pattern_keys:
        pattern_name = key.split("/")[-1]
        pattern_data = await state_manager.load_state(key)
        patterns.append({
            "name": pattern_name,
            "actions": len(pattern_data) if pattern_data else 0
        })
        
    return patterns


@app.post("/patterns/{pattern_name}/execute")
async def execute_pattern(pattern_name: str):
    """Execute a saved pattern."""
    if not state_manager:
        raise HTTPException(status_code=503, detail="State manager not initialized")
        
    actions = await state_manager.load_pattern(pattern_name)
    if not actions:
        raise HTTPException(status_code=404, detail=f"Pattern '{pattern_name}' not found")
        
    # Execute pattern
    operations = [Operation(action=action) for action in actions]
    results = await orchestrator.execute_sequence(operations)
    
    return {
        "pattern": pattern_name,
        "results": [
            {
                "operation_id": r.operation_id,
                "status": r.status.value,
                "error": str(r.error) if r.error else None
            }
            for r in results
        ]
    }


@app.post("/security/sandbox/{enabled}")
async def set_sandbox_mode(enabled: bool):
    """Enable/disable sandbox mode."""
    if not security_guardian:
        raise HTTPException(status_code=503, detail="Security guardian not initialized")
        
    security_guardian.context.sandbox_mode = enabled
    
    await manager.broadcast(json.dumps({
        "type": "security_mode_changed",
        "sandbox_mode": enabled,
        "timestamp": datetime.utcnow().isoformat()
    }))
    
    return {"sandbox_mode": enabled}


@app.get("/security/rules")
async def get_security_rules():
    """Get security rules."""
    if not security_guardian:
        return []
        
    rules = await security_guardian.export_rules()
    return rules


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket endpoint for real-time updates."""
    await manager.connect(websocket)
    
    try:
        # Send initial status
        status = await get_status()
        await websocket.send_json({
            "type": "status",
            "data": status.dict()
        })
        
        while True:
            # Receive messages
            data = await websocket.receive_text()
            message = json.loads(data)
            
            if message["type"] == "screenshot":
                # Send current screenshot
                # This would integrate with the screenshot capture
                pass
                
            elif message["type"] == "execute":
                # Execute operation
                request = OperationRequest(**message["data"])
                result = await execute_operation(request)
                await websocket.send_json({
                    "type": "execution_result",
                    "data": result
                })
                
    except WebSocketDisconnect:
        manager.disconnect(websocket)
        logger.info("WebSocket client disconnected")


@app.get("/screenshot/current")
async def get_current_screenshot():
    """Get current screenshot."""
    try:
        # This would integrate with the screenshot capture
        # For now, return a placeholder
        return {
            "screenshot": None,
            "timestamp": datetime.utcnow().isoformat()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/cache/clear")
async def clear_cache():
    """Clear performance cache."""
    if performance_optimizer:
        await performance_optimizer.clear_cache()
        return {"message": "Cache cleared"}
    return {"message": "No cache to clear"}


# Mount static files for web UI
@app.on_event("startup")
async def mount_static():
    """Mount static files."""
    try:
        app.mount("/static", StaticFiles(directory="web/build"), name="static")
    except:
        logger.warning("Static files not found, web UI will not be available")


def run_server(host: str = "0.0.0.0", port: int = 8000):
    """Run the API server."""
    uvicorn.run(app, host=host, port=port)