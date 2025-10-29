use anyhow::Result;
use clap::{Parser, Subcommand};
use llama_operate::{LlamaOperateSystem, SystemConfig};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "llama-operate")]
#[command(about = "Unified AI-powered development platform", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the complete system
    Start {
        /// Enable debug mode
        #[arg(short, long)]
        debug: bool,
    },
    
    /// Manage repositories
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },
    
    /// Manage workflows
    Workflow {
        #[command(subcommand)]
        action: WorkflowAction,
    },
    
    /// System monitoring
    Monitor {
        /// Show system status
        #[arg(short, long)]
        status: bool,
        
        /// Show metrics
        #[arg(short, long)]
        metrics: bool,
    },
    
    /// Run specific task
    Task {
        /// Task type
        #[arg(short, long)]
        task_type: String,
        
        /// Repository
        #[arg(short, long)]
        repo: String,
        
        /// Task parameters (JSON)
        #[arg(short, long)]
        params: Option<String>,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    /// Add a repository
    Add {
        /// Repository full name (owner/repo)
        name: String,
    },
    
    /// List repositories
    List,
    
    /// Analyze a repository
    Analyze {
        /// Repository name
        name: String,
    },
}

#[derive(Subcommand)]
enum WorkflowAction {
    /// List available workflows
    List,
    
    /// Trigger a workflow
    Trigger {
        /// Workflow ID
        id: String,
        
        /// Repository
        #[arg(short, long)]
        repo: String,
    },
    
    /// Show workflow status
    Status {
        /// Instance ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize telemetry
    llama_operate::telemetry::init_telemetry()?;
    
    let cli = Cli::parse();
    
    // Load configuration
    let config_content = tokio::fs::read_to_string(&cli.config).await?;
    let config: SystemConfig = toml::from_str(&config_content)?;
    
    match cli.command {
        Commands::Start { debug } => {
            if debug {
                std::env::set_var("RUST_LOG", "debug");
            }
            
            info!("Starting Llama-Operate system...");
            
            // Initialize system
            let system = LlamaOperateSystem::new(config).await?;
            
            // Start all subsystems
            system.start().await?;
            
            info!("System started successfully");
            
            // Keep running
            tokio::signal::ctrl_c().await?;
            info!("Shutting down...");
        },
        
        Commands::Repo { action } => {
            let system = LlamaOperateSystem::new(config).await?;
            
            match action {
                RepoAction::Add { name } => {
                    info!("Adding repository: {}", name);
                    
                    let parts: Vec<&str> = name.split('/').collect();
                    if parts.len() != 2 {
                        anyhow::bail!("Invalid repository format. Use owner/repo");
                    }
                    
                    system.command_center.register_repository(
                        llama_operate::command_center::Repository {
                            full_name: name.clone(),
                            owner: parts[0].to_string(),
                            name: parts[1].to_string(),
                            description: None,
                            language: None,
                            stars: 0,
                            last_sync: chrono::Utc::now(),
                        }
                    ).await?;
                    
                    info!("Repository added successfully");
                },
                
                RepoAction::List => {
                    // Implementation would list repositories
                    info!("Listing repositories...");
                },
                
                RepoAction::Analyze { name } => {
                    info!("Analyzing repository: {}", name);
                    
                    let task = system.command_center.create_task(
                        llama_operate::command_center::TaskRequest {
                            task_type: "analyze_code".to_string(),
                            repository: name,
                            title: "Code analysis".to_string(),
                            description: "Full repository analysis".to_string(),
                        }
                    ).await?;
                    
                    let result = system.command_center.execute_task(&task.id).await?;
                    
                    println!("Analysis completed: {}", serde_json::to_string_pretty(&result)?);
                },
            }
        },
        
        Commands::Workflow { action } => {
            let system = LlamaOperateSystem::new(config).await?;
            
            match action {
                WorkflowAction::List => {
                    info!("Available workflows:");
                    println!("- code_review: Automated Code Review");
                    println!("- bug_fix: Automated Bug Fix");
                    println!("- continuous_improvement: Continuous Code Improvement");
                },
                
                WorkflowAction::Trigger { id, repo } => {
                    info!("Triggering workflow: {}", id);
                    
                    let instance_id = system.workflow_engine.trigger_workflow(
                        &id,
                        llama_operate::workflows::WorkflowContext {
                            repository: repo,
                            trigger: llama_operate::workflows::WorkflowTrigger::Manual,
                            metadata: serde_json::json!({}),
                        },
                    ).await?;
                    
                    println!("Workflow triggered with instance ID: {}", instance_id);
                },
                
                WorkflowAction::Status { id } => {
                    info!("Checking workflow status: {}", id);
                    // Implementation would check workflow status
                },
            }
        },
        
        Commands::Monitor { status, metrics } => {
            let system = LlamaOperateSystem::new(config).await?;
            
            if status {
                let system_status = system.monitor.get_system_status().await;
                println!("System Status: {}", serde_json::to_string_pretty(&system_status)?);
            }
            
            if metrics {
                let cpu_metrics = system.monitor.get_metrics(
                    "system.cpu",
                    chrono::Duration::minutes(10)
                ).await;
                
                println!("CPU Metrics: {}", serde_json::to_string_pretty(&cpu_metrics)?);
            }
        },
        
        Commands::Task { task_type, repo, params } => {
            let system = LlamaOperateSystem::new(config).await?;
            
            info!("Creating task: {}", task_type);
            
            let task = system.command_center.create_task(
                llama_operate::command_center::TaskRequest {
                    task_type,
                    repository: repo,
                    title: "Manual task".to_string(),
                    description: params.unwrap_or_default(),
                }
            ).await?;
            
            info!("Executing task: {}", task.id);
            
            let result = system.command_center.execute_task(&task.id).await?;
            
            println!("Task completed: {}", serde_json::to_string_pretty(&result)?);
        },
    }
    
    Ok(())
}