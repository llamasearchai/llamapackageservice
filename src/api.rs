use crate::config::Config;
use crate::error::{ProcessorError, Result};
use crate::processors::{ProcessorFactory, PackageProcessor};
use crate::output_organizer::{self, OutputPaths};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Request payload for processing a package
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRequest {
    /// The URL or local path to process
    pub url: String,
    /// Optional custom output directory
    pub output_dir: Option<String>,
    /// Optional configuration overrides
    pub config: Option<ProcessConfig>,
}

/// Configuration options for processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessConfig {
    /// Whether to generate an index file
    pub generate_index: Option<bool>,
    /// Whether to organize output files
    pub organize_output: Option<bool>,
    /// Maximum number of concurrent operations
    pub max_concurrent: Option<usize>,
}

/// Response for a processing request
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResponse {
    /// Unique job ID for tracking the request
    pub job_id: String,
    /// Status of the processing job
    pub status: String,
    /// URL type detected
    pub url_type: String,
    /// Output directory where files will be saved
    pub output_dir: String,
    /// Message about the processing status
    pub message: String,
}

/// Job status information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobStatus {
    /// Unique job identifier
    pub job_id: String,
    /// Current status of the job
    pub status: JobStatusType,
    /// URL being processed
    pub url: String,
    /// Type of URL detected
    pub url_type: String,
    /// Output directory for the job
    pub output_dir: PathBuf,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Current operation being performed
    pub current_operation: Option<String>,
    /// Any error message if the job failed
    pub error_message: Option<String>,
    /// List of output files generated
    pub output_files: Vec<String>,
}

/// Possible job status types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatusType {
    /// Job is queued and waiting to be processed
    Queued,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
    /// Job was cancelled
    Cancelled,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Current status
    pub status: String,
    /// Current timestamp
    pub timestamp: DateTime<Utc>,
    /// Service uptime in seconds
    pub uptime: u64,
    /// Number of active jobs
    pub active_jobs: usize,
    /// Number of completed jobs
    pub completed_jobs: usize,
}

/// Job manager for tracking processing jobs
pub struct JobManager {
    /// Map of job ID to job status
    jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
    /// Configuration for the service
    config: Arc<Config>,
    /// Service start time for uptime calculation
    start_time: DateTime<Utc>,
}

impl JobManager {
    /// Create a new job manager
    pub fn new(config: Config) -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(config),
            start_time: Utc::now(),
        }
    }

    /// Submit a new processing job
    pub async fn submit_job(&self, request: ProcessRequest) -> Result<ProcessResponse> {
        use uuid::Uuid;
        let job_id = Uuid::new_v4().to_string();
        // Normalize user input to handle trailing spaces and quoted paths for local processing
        let mut request = request;
        let normalized_url = crate::utils::normalize_url_or_path(&request.url);
        request.url = normalized_url;
        let url_type = ProcessorFactory::detect_url_type(&request.url);
        
        // Validate the URL first
        let processor = ProcessorFactory::create_processor(&request.url)?;
        processor.validate(&request.url).await?;

        let output_dir = if let Some(ref custom_dir) = request.output_dir {
            PathBuf::from(custom_dir)
        } else {
            self.config.output_dir.clone()
        };

        let job_status = JobStatus {
            job_id: job_id.clone(),
            status: JobStatusType::Queued,
            url: request.url.clone(),
            url_type: url_type.clone(),
            output_dir: output_dir.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            progress: 0,
            current_operation: Some("Validating URL".to_string()),
            error_message: None,
            output_files: Vec::new(),
        };

        // Store the job
        {
            let mut jobs = self.jobs.lock().await;
            jobs.insert(job_id.clone(), job_status);
        }

        // Start processing in background
        let jobs_clone = Arc::clone(&self.jobs);
        let config_clone = Arc::clone(&self.config);
        let job_id_clone = job_id.clone();
        let request_clone = request.clone();
        
        tokio::spawn(async move {
            Self::process_job(jobs_clone, config_clone, job_id_clone, request_clone).await;
        });

        Ok(ProcessResponse {
            job_id,
            status: "queued".to_string(),
            url_type,
            output_dir: output_dir.to_string_lossy().to_string(),
            message: "Job queued for processing".to_string(),
        })
    }

    /// Get the status of a job
    pub async fn get_job_status(&self, job_id: &str) -> Result<JobStatus> {
        let jobs = self.jobs.lock().await;
        jobs.get(job_id)
            .cloned()
            .ok_or_else(|| ProcessorError::Message(format!("Job not found: {}", job_id)))
    }

    /// Get output directory
    pub fn output_dir(&self) -> &std::path::Path {
        &self.config.output_dir
    }

    /// Get service health information
    pub async fn get_health(&self) -> HealthResponse {
        let jobs = self.jobs.lock().await;
        let active_jobs = jobs.values()
            .filter(|job| job.status == JobStatusType::Processing || job.status == JobStatusType::Queued)
            .count();
        let completed_jobs = jobs.values()
            .filter(|job| job.status == JobStatusType::Completed)
            .count();

        HealthResponse {
            service: "LlamaPackageService".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: "healthy".to_string(),
            timestamp: Utc::now(),
            uptime: (Utc::now() - self.start_time).num_seconds() as u64,
            active_jobs,
            completed_jobs,
        }
    }

    /// Internal method to process a job
    async fn process_job(
        jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
        config: Arc<Config>,
        job_id: String,
        request: ProcessRequest,
    ) {
        // Update job status to processing
        {
            let mut jobs_guard = jobs.lock().await;
            if let Some(job) = jobs_guard.get_mut(&job_id) {
                job.status = JobStatusType::Processing;
                job.updated_at = Utc::now();
                job.progress = 10;
                job.current_operation = Some("Starting processing".to_string());
            }
        }

        // Determine output directory
        let output_dir = if let Some(custom_dir) = request.output_dir {
            PathBuf::from(custom_dir)
        } else {
            config.output_dir.clone()
        };

        // Process the request
        let result = async {
            // Create processor
            let processor = ProcessorFactory::create_processor(&request.url)?;
            
            // Update progress
            {
                let mut jobs_guard = jobs.lock().await;
                if let Some(job) = jobs_guard.get_mut(&job_id) {
                    job.progress = 30;
                    job.current_operation = Some("Processing package".to_string());
                }
            }

            // Process the package
            processor.process(&request.url, &output_dir, &config).await?;

            // Update progress
            {
                let mut jobs_guard = jobs.lock().await;
                if let Some(job) = jobs_guard.get_mut(&job_id) {
                    job.progress = 80;
                    job.current_operation = Some("Organizing output".to_string());
                }
            }

            // Organize output if requested
            if request.config.as_ref()
                .and_then(|c| c.organize_output)
                .unwrap_or(true) {
                if let Err(e) = output_organizer::organize_output(&output_dir) {
                    eprintln!("Warning: Failed to organize output: {}", e);
                }
            }

            Ok::<(), ProcessorError>(())
        }.await;

        // Update final job status
        {
            let mut jobs_guard = jobs.lock().await;
            if let Some(job) = jobs_guard.get_mut(&job_id) {
                match result {
                    Ok(_) => {
                        job.status = JobStatusType::Completed;
                        job.progress = 100;
                        job.current_operation = Some("Completed successfully".to_string());
                    },
                    Err(e) => {
                        job.status = JobStatusType::Failed;
                        job.error_message = Some(e.to_string());
                        job.current_operation = Some("Failed".to_string());
                    }
                }
                job.updated_at = Utc::now();
            }
        }
    }
}

impl Clone for ProcessRequest {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            output_dir: self.output_dir.clone(),
            config: self.config.clone(),
        }
    }
}

/// AI Analysis endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRequest {
    /// Repository URL to analyze
    pub repository: String,
    /// Type of analysis to perform
    pub analysis_type: String,
    /// Optional context for the analysis
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResponse {
    /// Analysis ID
    pub id: String,
    /// Analysis result
    pub result: String,
    /// Confidence score
    pub confidence: f32,
    /// Analysis type
    pub analysis_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationRequest {
    /// Repository to discuss
    pub repository: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    /// Conversation ID
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRequest {
    /// Conversation ID
    pub conversation_id: String,
    /// Message to send
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    /// AI response
    pub response: String,
}

/// Analyze repository using AI
pub async fn analyze_repository(request: AnalysisRequest) -> Result<AnalysisResponse> {
    use crate::agents::{OpenAIAgent, AnalysisRequest as AgentRequest, AnalysisType};
    
    // Create OpenAI agent
    let agent = OpenAIAgent::from_env().map_err(|e| {
        ProcessorError::new(&format!("Failed to create OpenAI agent: {}", e))
    })?;
    
    // Parse analysis type
    let analysis_type = match request.analysis_type.as_str() {
        "documentation" => AnalysisType::Documentation,
        "security" => AnalysisType::SecurityAudit,
        "code_review" => AnalysisType::CodeReview,
        "examples" => AnalysisType::Examples,
        "api_docs" => AnalysisType::ApiDocumentation,
        custom => AnalysisType::Custom(custom.to_string()),
    };
    
    // Create analysis request
    let agent_request = AgentRequest {
        repository: request.repository,
        analysis_type,
        context: request.context,
        parameters: std::collections::HashMap::new(),
    };
    
    // Perform analysis
    let result = agent.analyze_repository(agent_request).await?;
    
    Ok(AnalysisResponse {
        id: result.id,
        result: result.content,
        confidence: result.confidence,
        analysis_type: request.analysis_type,
    })
}

/// Start a conversation about a repository
pub async fn start_conversation(request: ConversationRequest) -> Result<ConversationResponse> {
    use crate::agents::OpenAIAgent;
    
    // Create OpenAI agent
    let agent = OpenAIAgent::from_env().map_err(|e| {
        ProcessorError::new(&format!("Failed to create OpenAI agent: {}", e))
    })?;
    
    // Start conversation
    let context = agent.start_conversation(request.repository).await?;
    
    Ok(ConversationResponse {
        conversation_id: context.id,
    })
}

/// Send a message in a conversation
pub async fn send_message(_request: MessageRequest) -> Result<MessageResponse> {
    // For now, return a placeholder response
    // In a full implementation, this would manage conversation state
    Err(ProcessorError::new("Conversation management not yet implemented"))
}
