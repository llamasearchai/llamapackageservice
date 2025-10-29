/// Python bridge for Operate Enhanced integration
use anyhow::{Result, Context};
use pyo3::prelude::*;
use pyo3_asyncio;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
pub struct PythonConfig {
    pub operate_path: String,
    pub model: String,
    pub sandbox_mode: bool,
    pub cache_enabled: bool,
}

/// Bridge to Python Operate Enhanced framework
pub struct PythonBridge {
    config: PythonConfig,
    py_app: Arc<RwLock<Option<PyObject>>>,
}

impl PythonBridge {
    pub async fn new(config: &PythonConfig) -> Result<Self> {
        let config = config.clone();
        
        // Initialize Python environment
        let py_app = Python::with_gil(|py| -> PyResult<PyObject> {
            // Add operate path to Python path
            let sys = py.import("sys")?;
            let path: &PyList = sys.getattr("path")?.downcast()?;
            path.insert(0, &config.operate_path)?;
            
            // Import operate module
            let operate_module = py.import("operate.main")?;
            let app_class = operate_module.getattr("OperateApp")?;
            
            // Create app instance
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("model", &config.model)?;
            kwargs.set_item("sandbox", config.sandbox_mode)?;
            kwargs.set_item("cache_enabled", config.cache_enabled)?;
            
            let app = app_class.call((), Some(kwargs))?;
            
            // Initialize the app
            let init_coro = app.call_method0("initialize")?;
            pyo3_asyncio::tokio::into_future(init_coro)?;
            
            Ok(app.into())
        }).context("Failed to initialize Python bridge")?;
        
        Ok(Self {
            config,
            py_app: Arc::new(RwLock::new(Some(py_app))),
        })
    }
    
    /// Execute an operation using Operate Enhanced
    pub async fn execute_operation(&self, operation: Operation) -> Result<OperationResult> {
        let py_app = self.py_app.read().await;
        let py_app = py_app.as_ref().context("Python app not initialized")?;
        
        Python::with_gil(|py| -> PyResult<OperationResult> {
            // Convert operation to Python dict
            let op_dict = pyo3::types::PyDict::new(py);
            op_dict.set_item("type", operation.action_type)?;
            op_dict.set_item("target", operation.target)?;
            op_dict.set_item("value", operation.value)?;
            op_dict.set_item("metadata", operation.metadata)?;
            
            // Call execute method
            let result_coro = py_app.call_method1(py, "execute_operation", (op_dict,))?;
            let result = pyo3_asyncio::tokio::into_future(result_coro)?;
            
            // Parse result
            let result_dict: &pyo3::types::PyDict = result.downcast(py)?;
            let status = result_dict.get_item("status")
                .and_then(|s| s.extract::<String>().ok())
                .unwrap_or_else(|| "unknown".to_string());
            
            let error = result_dict.get_item("error")
                .and_then(|e| e.extract::<String>().ok());
            
            Ok(OperationResult {
                operation_id: operation.id,
                status,
                error,
                data: None,
            })
        }).context("Failed to execute operation")
    }
    
    /// Capture and analyze screenshot
    pub async fn analyze_screen(&self, objective: &str) -> Result<Action> {
        let py_app = self.py_app.read().await;
        let py_app = py_app.as_ref().context("Python app not initialized")?;
        
        Python::with_gil(|py| -> PyResult<Action> {
            // Capture screenshot
            let screenshot_coro = py_app.call_method0(py, "capture_screenshot")?;
            let screenshot = pyo3_asyncio::tokio::into_future(screenshot_coro)?;
            
            // Analyze with AI
            let analyze_coro = py_app.call_method1(
                py, 
                "analyze_screen", 
                (screenshot, objective)
            )?;
            let action = pyo3_asyncio::tokio::into_future(analyze_coro)?;
            
            // Convert to Rust type
            let action_dict: &pyo3::types::PyDict = action.downcast(py)?;
            
            Ok(Action {
                action_type: action_dict.get_item("type")
                    .and_then(|t| t.extract().ok())
                    .unwrap_or_else(|| "wait".to_string()),
                target: action_dict.get_item("target")
                    .and_then(|t| t.extract().ok()),
                value: action_dict.get_item("value")
                    .and_then(|v| v.extract().ok()),
                reasoning: action_dict.get_item("reasoning")
                    .and_then(|r| r.extract().ok()),
            })
        }).context("Failed to analyze screen")
    }
    
    /// Execute GitHub operation
    pub async fn github_operation(&self, op: GitHubOperation) -> Result<serde_json::Value> {
        let py_app = self.py_app.read().await;
        let py_app = py_app.as_ref().context("Python app not initialized")?;
        
        Python::with_gil(|py| -> PyResult<serde_json::Value> {
            // Get GitHub manager
            let github_manager = py_app.getattr(py, "github_manager")?;
            
            // Execute operation
            let result = match op {
                GitHubOperation::AnalyzeRepo { repo_path } => {
                    let analyze_coro = github_manager.call_method1(
                        py,
                        "analyze_codebase",
                        (repo_path,)
                    )?;
                    pyo3_asyncio::tokio::into_future(analyze_coro)?
                },
                GitHubOperation::CreatePR { repo, title, body, head, base } => {
                    let pr_coro = github_manager.call_method(
                        py,
                        "create_pull_request",
                        (repo, title, body, head, base),
                        None
                    )?;
                    pyo3_asyncio::tokio::into_future(pr_coro)?
                },
                GitHubOperation::ReviewPR { repo, pr_number } => {
                    let review_coro = github_manager.call_method1(
                        py,
                        "review_pull_request",
                        (repo, pr_number)
                    )?;
                    pyo3_asyncio::tokio::into_future(review_coro)?
                },
            };
            
            // Convert result to JSON
            let json_str = result.call_method0(py, "__str__")?
                .extract::<String>(py)?;
            
            Ok(serde_json::from_str(&json_str)?)
        }).context("Failed to execute GitHub operation")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub action_type: String,
    pub target: Option<String>,
    pub value: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation_id: String,
    pub status: String,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub target: Option<String>,
    pub value: Option<serde_json::Value>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone)]
pub enum GitHubOperation {
    AnalyzeRepo { repo_path: String },
    CreatePR { 
        repo: String,
        title: String,
        body: String,
        head: String,
        base: String,
    },
    ReviewPR {
        repo: String,
        pr_number: i32,
    },
}