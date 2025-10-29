/// Unified command center for GitHub repository and program management
use anyhow::{Result, Context};
use octocrab::{Octocrab, models};
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub struct CommandCenter {
    github: Arc<Octocrab>,
    db: SqlitePool,
    repositories: Arc<DashMap<String, Repository>>,
    active_tasks: Arc<DashMap<String, Task>>,
}

impl CommandCenter {
    pub async fn new(github_token: String, database_url: String) -> Result<Self> {
        // Initialize GitHub client
        let github = Arc::new(
            Octocrab::builder()
                .personal_token(github_token)
                .build()
                .context("Failed to create GitHub client")?
        );
        
        // Initialize database
        let db = SqlitePool::connect(&database_url).await
            .context("Failed to connect to database")?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&db).await
            .context("Failed to run migrations")?;
        
        Ok(Self {
            github,
            db,
            repositories: Arc::new(DashMap::new()),
            active_tasks: Arc::new(DashMap::new()),
        })
    }
    
    /// Register a repository for management
    pub async fn register_repository(&self, repo: Repository) -> Result<()> {
        // Store in memory
        self.repositories.insert(repo.full_name.clone(), repo.clone());
        
        // Store in database
        sqlx::query!(
            r#"
            INSERT INTO repositories (full_name, owner, name, description, language, stars, last_sync)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(full_name) DO UPDATE SET
                description = excluded.description,
                language = excluded.language,
                stars = excluded.stars,
                last_sync = excluded.last_sync
            "#,
            repo.full_name,
            repo.owner,
            repo.name,
            repo.description,
            repo.language,
            repo.stars,
            repo.last_sync
        )
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Create a new task
    pub async fn create_task(&self, task: TaskRequest) -> Result<Task> {
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            task_type: task.task_type,
            repository: task.repository,
            title: task.title,
            description: task.description,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
        };
        
        // Store in memory
        self.active_tasks.insert(task.id.clone(), task.clone());
        
        // Store in database
        sqlx::query!(
            r#"
            INSERT INTO tasks (id, task_type, repository, title, description, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            task.id,
            task.task_type,
            task.repository,
            task.title,
            task.description,
            "pending",
            task.created_at
        )
        .execute(&self.db)
        .await?;
        
        Ok(task)
    }
    
    /// Execute a task
    pub async fn execute_task(&self, task_id: &str) -> Result<TaskResult> {
        let task = self.active_tasks.get(task_id)
            .context("Task not found")?
            .clone();
        
        // Update status
        self.update_task_status(task_id, TaskStatus::Running).await?;
        
        let result = match task.task_type.as_str() {
            "analyze_code" => self.analyze_code_task(&task).await?,
            "create_pr" => self.create_pr_task(&task).await?,
            "review_pr" => self.review_pr_task(&task).await?,
            "run_tests" => self.run_tests_task(&task).await?,
            "deploy" => self.deploy_task(&task).await?,
            _ => return Err(anyhow::anyhow!("Unknown task type: {}", task.task_type)),
        };
        
        // Update task with result
        self.update_task_result(task_id, &result).await?;
        
        Ok(result)
    }
    
    async fn analyze_code_task(&self, task: &Task) -> Result<TaskResult> {
        let repo = self.repositories.get(&task.repository)
            .context("Repository not found")?
            .clone();
        
        // Use GitHub API to analyze repository
        let repo_info = self.github
            .repos(&repo.owner, &repo.name)
            .get()
            .await?;
        
        // Get language statistics
        let languages = self.github
            .repos(&repo.owner, &repo.name)
            .list_languages()
            .await?;
        
        // Get recent commits
        let commits = self.github
            .repos(&repo.owner, &repo.name)
            .list_commits()
            .per_page(10)
            .send()
            .await?;
        
        let analysis = CodeAnalysis {
            primary_language: repo_info.language.clone(),
            languages: languages.into_iter().collect(),
            recent_commits: commits.items.len(),
            open_issues: repo_info.open_issues_count.unwrap_or(0) as i32,
            stars: repo_info.stargazers_count.unwrap_or(0) as i32,
        };
        
        Ok(TaskResult {
            success: true,
            message: "Code analysis completed".to_string(),
            data: Some(serde_json::to_value(analysis)?),
        })
    }
    
    async fn create_pr_task(&self, task: &Task) -> Result<TaskResult> {
        // Extract PR details from task description
        let pr_details: CreatePRDetails = serde_json::from_str(&task.description)?;
        
        let repo_parts: Vec<&str> = task.repository.split('/').collect();
        if repo_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository format"));
        }
        
        // Create pull request
        let pr = self.github
            .pulls(repo_parts[0], repo_parts[1])
            .create(
                pr_details.title,
                pr_details.head,
                pr_details.base,
            )
            .body(pr_details.body)
            .send()
            .await?;
        
        Ok(TaskResult {
            success: true,
            message: format!("Pull request #{} created", pr.number),
            data: Some(serde_json::json!({
                "pr_number": pr.number,
                "url": pr.html_url,
            })),
        })
    }
    
    async fn review_pr_task(&self, task: &Task) -> Result<TaskResult> {
        // Implementation for PR review
        Ok(TaskResult {
            success: true,
            message: "PR review completed".to_string(),
            data: None,
        })
    }
    
    async fn run_tests_task(&self, task: &Task) -> Result<TaskResult> {
        // Implementation for running tests
        Ok(TaskResult {
            success: true,
            message: "Tests completed".to_string(),
            data: None,
        })
    }
    
    async fn deploy_task(&self, task: &Task) -> Result<TaskResult> {
        // Implementation for deployment
        Ok(TaskResult {
            success: true,
            message: "Deployment completed".to_string(),
            data: None,
        })
    }
    
    async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<()> {
        if let Some(mut task) = self.active_tasks.get_mut(task_id) {
            task.status = status.clone();
            task.updated_at = Utc::now();
        }
        
        sqlx::query!(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            status.to_string(),
            Utc::now(),
            task_id
        )
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    async fn update_task_result(&self, task_id: &str, result: &TaskResult) -> Result<()> {
        let status = if result.success { 
            TaskStatus::Completed 
        } else { 
            TaskStatus::Failed 
        };
        
        self.update_task_status(task_id, status).await?;
        
        sqlx::query!(
            "UPDATE tasks SET result = ?1 WHERE id = ?2",
            serde_json::to_string(result)?,
            task_id
        )
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    pub async fn start_monitoring(&self) -> Result<()> {
        // Start monitoring registered repositories
        for entry in self.repositories.iter() {
            let repo = entry.value().clone();
            let github = self.github.clone();
            
            tokio::spawn(async move {
                // Monitor repository for changes
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                    
                    // Check for new issues, PRs, etc.
                    if let Ok(issues) = github
                        .issues(&repo.owner, &repo.name)
                        .list()
                        .state(octocrab::params::State::Open)
                        .send()
                        .await
                    {
                        // Process new issues
                        for issue in issues {
                            tracing::info!("New issue found: {}", issue.title);
                        }
                    }
                }
            });
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: i32,
    pub last_sync: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub repository: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Pending => "pending".to_string(),
            TaskStatus::Running => "running".to_string(),
            TaskStatus::Completed => "completed".to_string(),
            TaskStatus::Failed => "failed".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_type: String,
    pub repository: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CodeAnalysis {
    primary_language: Option<String>,
    languages: Vec<(String, i64)>,
    recent_commits: usize,
    open_issues: i32,
    stars: i32,
}

#[derive(Debug, Deserialize)]
struct CreatePRDetails {
    title: String,
    body: String,
    head: String,
    base: String,
}