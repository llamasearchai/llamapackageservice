use llamapackageservice::{Config, api::{JobManager, ProcessRequest, AnalysisRequest, ConversationRequest, MessageRequest}};
use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use axum_extra::routing::RouterExt;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn, error};

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    job_manager: Arc<JobManager>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();
    
    // Create configuration
    let output_dir = std::env::var("OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./output"));
    
    let config = Config::new(output_dir);
    config.ensure_directories_exist().await?;
    
    // Create job manager
    let job_manager = Arc::new(JobManager::new(config));
    let state = AppState { job_manager };
    
    info!("LlamaPackageService Web Server Starting...");
    info!("Output directory: {}", state.job_manager.output_dir().display());
    info!("Server will be available at http://localhost:8000");
    info!("API documentation: http://localhost:8000/docs");
    info!("Health check: http://localhost:8000/health");
    
    // Build application routes
    let app = create_app(state);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    info!("Server listening on http://127.0.0.1:8000");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Create the main application with all routes
fn create_app(state: AppState) -> Router {
    Router::new()
        // Health and status endpoints
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/api/health", get(health_check))
        .route("/status", get(status))
        
        // Processing endpoints
        .route("/api/process", post(process_repository))
        .route("/api/jobs/:job_id", get(get_job_status))
        .route("/api/jobs", get(list_jobs))
        
        // AI Analysis endpoints
        .route("/api/analyze", post(analyze_repository))
        .route("/api/conversation", post(start_conversation))
        .route("/api/conversation/:conversation_id/message", post(send_message))
        
        // Documentation
        .route("/docs", get(api_docs))
        .route("/openapi.json", get(openapi_spec))
        
        // Add middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Root endpoint - returns basic service information
async fn index() -> ResponseJson<Value> {
    ResponseJson(json!({
        "service": "LlamaPackageService",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Transform code repositories into structured text representations",
        "author": "Nik Jois <nikjois@llamasearch.ai>",
        "endpoints": {
            "health": "/health",
            "process": "/api/process",
            "analyze": "/api/analyze",
            "conversation": "/api/conversation",
            "documentation": "/docs"
        }
    }))
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> ResponseJson<Value> {
    let health = state.job_manager.get_health().await;
    ResponseJson(json!(health))
}

/// Service status endpoint
async fn status(State(state): State<AppState>) -> ResponseJson<Value> {
    let health = state.job_manager.get_health().await;
    ResponseJson(json!({
        "status": "operational",
        "uptime_seconds": health.uptime,
        "active_jobs": health.active_jobs,
        "completed_jobs": health.completed_jobs,
        "memory_usage": get_memory_usage(),
        "rust_version": std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        "build_time": std::env::var("BUILD_TIME").unwrap_or_else(|_| "unknown".to_string())
    }))
}

/// Process a repository endpoint
async fn process_repository(
    State(state): State<AppState>,
    Json(request): Json<ProcessRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    info!("Processing repository: {}", request.url);
    
    match state.job_manager.submit_job(request).await {
        Ok(response) => {
            info!("Job submitted successfully: {}", response.job_id);
            Ok(ResponseJson(json!(response)))
        },
        Err(e) => {
            error!("Failed to submit job: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get job status endpoint
async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<ResponseJson<Value>, StatusCode> {
    match state.job_manager.get_job_status(&job_id).await {
        Ok(status) => Ok(ResponseJson(json!(status))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List all jobs endpoint
async fn list_jobs(State(_state): State<AppState>) -> ResponseJson<Value> {
    // This would need to be implemented in JobManager
    ResponseJson(json!({
        "jobs": [],
        "total": 0,
        "message": "Job listing not yet implemented"
    }))
}

/// Analyze repository with AI endpoint
async fn analyze_repository(
    State(_state): State<AppState>,
    Json(request): Json<AnalysisRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    info!("AI analysis requested for repository: {}", request.repository);
    
    // This would use the OpenAI agents integration
    match llamapackageservice::api::analyze_repository(request).await {
        Ok(response) => Ok(ResponseJson(json!(response))),
        Err(e) => {
            error!("Analysis failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Start conversation endpoint
async fn start_conversation(
    State(_state): State<AppState>,
    Json(request): Json<ConversationRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    info!("Starting conversation for repository: {}", request.repository);
    
    match llamapackageservice::api::start_conversation(request).await {
        Ok(response) => Ok(ResponseJson(json!(response))),
        Err(e) => {
            error!("Failed to start conversation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Send message to conversation endpoint
async fn send_message(
    State(_state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(mut request): Json<MessageRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    info!("Sending message to conversation: {}", conversation_id);
    
    // Set conversation ID from path
    request.conversation_id = conversation_id;
    
    match llamapackageservice::api::send_message(request).await {
        Ok(response) => Ok(ResponseJson(json!(response))),
        Err(e) => {
            error!("Failed to send message: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// API documentation endpoint
async fn api_docs() -> &'static str {
    r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>LlamaPackageService API Documentation</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            h1 { color: #333; }
            h2 { color: #666; }
            code { background: #f4f4f4; padding: 2px 4px; border-radius: 3px; }
            pre { background: #f4f4f4; padding: 10px; border-radius: 5px; overflow-x: auto; }
            .endpoint { margin: 20px 0; padding: 15px; background: #f9f9f9; border-radius: 5px; }
        </style>
    </head>
    <body>
        <h1>LlamaPackageService API Documentation</h1>
        <p>Author: <strong>Nik Jois &lt;nikjois@llamasearch.ai&gt;</strong></p>
        
        <h2>Endpoints</h2>
        
        <div class="endpoint">
            <h3>GET /health</h3>
            <p>Health check endpoint</p>
            <pre>Response: {"service": "...", "status": "...", "uptime": 123}</pre>
        </div>
        
        <div class="endpoint">
            <h3>POST /api/process</h3>
            <p>Process a repository</p>
            <pre>Request: {"url": "https://github.com/user/repo", "output_dir": "/optional/path"}</pre>
            <pre>Response: {"job_id": "uuid", "status": "queued", "message": "..."}</pre>
        </div>
        
        <div class="endpoint">
            <h3>GET /api/jobs/{job_id}</h3>
            <p>Get job status</p>
            <pre>Response: {"job_id": "uuid", "status": "completed", "progress": 100}</pre>
        </div>
        
        <div class="endpoint">
            <h3>POST /api/analyze</h3>
            <p>AI-powered repository analysis</p>
            <pre>Request: {"repository": "https://github.com/user/repo", "analysis_type": "Documentation"}</pre>
            <pre>Response: {"id": "uuid", "result": "...", "confidence": 0.95}</pre>
        </div>
        
        <div class="endpoint">
            <h3>POST /api/conversation</h3>
            <p>Start AI conversation about repository</p>
            <pre>Request: {"repository": "https://github.com/user/repo"}</pre>
            <pre>Response: {"conversation_id": "uuid"}</pre>
        </div>
        
        <div class="endpoint">
            <h3>POST /api/conversation/{id}/message</h3>
            <p>Send message to AI conversation</p>
            <pre>Request: {"message": "What does this code do?"}</pre>
            <pre>Response: {"response": "This code implements..."}</pre>
        </div>
    </body>
    </html>
    "#
}

/// OpenAPI specification endpoint
async fn openapi_spec() -> ResponseJson<Value> {
    ResponseJson(json!({
        "openapi": "3.0.0",
        "info": {
            "title": "LlamaPackageService API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Transform code repositories into structured text representations",
            "contact": {
                "name": "Nik Jois",
                "email": "nikjois@llamasearch.ai"
            }
        },
        "servers": [
            {
                "url": "http://localhost:8000",
                "description": "Development server"
            }
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service health information"
                        }
                    }
                }
            },
            "/api/process": {
                "post": {
                    "summary": "Process repository",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "url": {"type": "string"},
                                        "output_dir": {"type": "string"}
                                    },
                                    "required": ["url"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Job submitted successfully"
                        }
                    }
                }
            }
        }
    }))
}

/// Get memory usage information
fn get_memory_usage() -> Value {
    // This is a placeholder - in a real implementation, you'd use system metrics
    json!({
        "rss_bytes": 0,
        "heap_bytes": 0,
        "message": "Memory metrics not implemented"
    })
}
