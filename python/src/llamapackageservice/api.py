"""
REST API for the LlamaPackageService.

Provides HTTP endpoints for programmatic access to package processing.
"""

from pathlib import Path
from typing import Optional, Dict, List
from datetime import datetime
from dataclasses import dataclass, field
from enum import Enum
import uuid
import asyncio
import logging

from pydantic import BaseModel

from .config import Config
from .error import ProcessorError
from .processors import ProcessorFactory
from .output_organizer import OutputPaths, list_output_files

logger = logging.getLogger(__name__)


# Request/Response models
class ProcessConfig(BaseModel):
    """Configuration options for processing."""
    generate_index: Optional[bool] = None
    organize_output: Optional[bool] = None
    max_concurrent: Optional[int] = None


class ProcessRequest(BaseModel):
    """Request payload for processing a package."""
    url: str
    output_dir: Optional[str] = None
    config: Optional[ProcessConfig] = None


class ProcessResponse(BaseModel):
    """Response for a processing request."""
    job_id: str
    status: str
    url_type: str
    output_dir: str
    message: str


class JobStatusType(str, Enum):
    """Possible job status types."""
    QUEUED = "queued"
    PROCESSING = "processing"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class JobStatus(BaseModel):
    """Job status information."""
    job_id: str
    status: JobStatusType
    url: str
    url_type: str
    output_dir: str
    created_at: datetime
    updated_at: datetime
    progress: int = 0
    current_operation: Optional[str] = None
    error_message: Optional[str] = None
    output_files: List[str] = []


class HealthResponse(BaseModel):
    """Health check response."""
    service: str
    version: str
    status: str
    timestamp: datetime
    uptime: int
    active_jobs: int
    completed_jobs: int


class JobManager:
    """Job manager for tracking processing jobs."""
    
    def __init__(self, config: Config):
        """Create a new job manager."""
        self._jobs: Dict[str, JobStatus] = {}
        self._config = config
        self._start_time = datetime.utcnow()
        self._completed_count = 0
        self._lock = asyncio.Lock()
    
    @property
    def output_dir(self) -> Path:
        """Get the output directory."""
        return self._config.output_dir
    
    async def submit_job(self, request: ProcessRequest) -> ProcessResponse:
        """Submit a new processing job."""
        job_id = str(uuid.uuid4())
        
        # Normalize URL
        from .utils import normalize_url_or_path
        normalized_url = normalize_url_or_path(request.url)
        url_type = ProcessorFactory.detect_url_type(normalized_url)
        
        # Validate URL
        processor = ProcessorFactory.create_processor(normalized_url)
        await processor.validate(normalized_url)
        
        output_dir = Path(request.output_dir) if request.output_dir else self._config.output_dir
        
        # Create job status
        job_status = JobStatus(
            job_id=job_id,
            status=JobStatusType.QUEUED,
            url=normalized_url,
            url_type=url_type,
            output_dir=str(output_dir),
            created_at=datetime.utcnow(),
            updated_at=datetime.utcnow(),
            progress=0,
            current_operation="Validating URL",
        )
        
        async with self._lock:
            self._jobs[job_id] = job_status
        
        # Start processing in background
        asyncio.create_task(self._process_job(job_id, normalized_url, output_dir))
        
        return ProcessResponse(
            job_id=job_id,
            status="queued",
            url_type=url_type,
            output_dir=str(output_dir),
            message="Job queued for processing",
        )
    
    async def get_job_status(self, job_id: str) -> JobStatus:
        """Get the status of a job."""
        async with self._lock:
            if job_id not in self._jobs:
                raise ProcessorError(f"Job not found: {job_id}")
            return self._jobs[job_id]
    
    async def list_jobs(self) -> List[JobStatus]:
        """List all jobs."""
        async with self._lock:
            return list(self._jobs.values())
    
    async def _process_job(
        self,
        job_id: str,
        url: str,
        output_dir: Path,
    ) -> None:
        """Process a job in the background."""
        try:
            # Update status to processing
            async with self._lock:
                self._jobs[job_id].status = JobStatusType.PROCESSING
                self._jobs[job_id].current_operation = "Starting processing"
                self._jobs[job_id].updated_at = datetime.utcnow()
            
            # Create processor and process
            processor = ProcessorFactory.create_processor(url)
            await processor.process(url, output_dir, self._config)
            
            # Update status to completed
            async with self._lock:
                self._jobs[job_id].status = JobStatusType.COMPLETED
                self._jobs[job_id].progress = 100
                self._jobs[job_id].current_operation = "Completed"
                self._jobs[job_id].updated_at = datetime.utcnow()
                self._completed_count += 1
                
        except Exception as e:
            logger.error(f"Job {job_id} failed: {e}")
            async with self._lock:
                self._jobs[job_id].status = JobStatusType.FAILED
                self._jobs[job_id].error_message = str(e)
                self._jobs[job_id].updated_at = datetime.utcnow()
    
    def get_health(self) -> HealthResponse:
        """Get health status."""
        uptime = int((datetime.utcnow() - self._start_time).total_seconds())
        active_jobs = sum(
            1 for job in self._jobs.values()
            if job.status in {JobStatusType.QUEUED, JobStatusType.PROCESSING}
        )
        
        return HealthResponse(
            service="LlamaPackageService",
            version="0.1.0",
            status="healthy",
            timestamp=datetime.utcnow(),
            uptime=uptime,
            active_jobs=active_jobs,
            completed_jobs=self._completed_count,
        )


# FastAPI application factory
def create_app(config: Optional[Config] = None):
    """Create the FastAPI application."""
    try:
        from fastapi import FastAPI, HTTPException
        from fastapi.middleware.cors import CORSMiddleware
    except ImportError:
        raise ImportError("FastAPI not installed. Run: pip install fastapi")
    
    if config is None:
        config = Config()
    
    app = FastAPI(
        title="LlamaPackageService API",
        description="REST API for package processing and analysis",
        version="0.1.0",
    )
    
    # Add CORS middleware
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    
    # Create job manager
    job_manager = JobManager(config)
    
    @app.get("/health")
    async def health_check() -> HealthResponse:
        """Health check endpoint."""
        return job_manager.get_health()
    
    @app.post("/api/process")
    async def submit_process_job(request: ProcessRequest) -> ProcessResponse:
        """Submit a new processing job."""
        try:
            return await job_manager.submit_job(request)
        except ProcessorError as e:
            raise HTTPException(status_code=400, detail=str(e))
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))
    
    @app.get("/api/status/{job_id}")
    async def get_job_status(job_id: str) -> JobStatus:
        """Get the status of a processing job."""
        try:
            return await job_manager.get_job_status(job_id)
        except ProcessorError as e:
            raise HTTPException(status_code=404, detail=str(e))
    
    @app.get("/api/jobs")
    async def list_jobs() -> List[JobStatus]:
        """List all jobs."""
        return await job_manager.list_jobs()
    
    @app.get("/api/output")
    async def list_output() -> Dict:
        """List output files."""
        paths = OutputPaths(config.output_dir)
        
        result = {}
        for category, dir_path in [
            ("github_repos", paths.github_repos_dir),
            ("github_orgs", paths.github_orgs_dir),
            ("pypi_packages", paths.pypi_packages_dir),
            ("npm_packages", paths.npm_packages_dir),
            ("rust_crates", paths.crates_dir),
            ("go_packages", paths.go_packages_dir),
            ("local_repos", paths.local_repos_dir),
        ]:
            if dir_path.exists():
                result[category] = [f.name for f in dir_path.iterdir() if f.is_file()]
            else:
                result[category] = []
        
        return result
    
    return app


async def start_server(config: Config, host: str = "0.0.0.0", port: int = 8000):
    """Start the API server."""
    try:
        import uvicorn
    except ImportError:
        raise ImportError("Uvicorn not installed. Run: pip install uvicorn")
    
    app = create_app(config)
    
    config_dict = uvicorn.Config(app, host=host, port=port, log_level="info")
    server = uvicorn.Server(config_dict)
    
    await server.serve()
