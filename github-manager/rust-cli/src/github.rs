use chrono::{DateTime, Utc};
use octocrab::{Octocrab, Page};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: Option<DateTime<Utc>>,
    pub homepage: Option<String>,
    pub size: u64,
    pub stars: u32,
    pub watchers: u32,
    pub forks: u32,
    pub open_issues_count: u32,
    pub language: Option<String>,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
    pub has_pages: bool,
    pub has_downloads: bool,
    pub archived: bool,
    pub disabled: bool,
    pub license: Option<License>,
    pub default_branch: String,
    pub url: String,
    pub clone_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub labels: Vec<Label>,
    pub assignees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub draft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub repository: String,
    pub trigger_type: String,
}

impl Default for Repository {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            full_name: String::new(),
            description: None,
            private: false,
            fork: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            pushed_at: None,
            homepage: None,
            size: 0,
            stars: 0,
            watchers: 0,
            forks: 0,
            open_issues_count: 0,
            language: None,
            has_issues: true,
            has_projects: true,
            has_wiki: true,
            has_pages: false,
            has_downloads: true,
            archived: false,
            disabled: false,
            license: None,
            default_branch: "main".to_string(),
            url: String::new(),
            clone_url: String::new(),
        }
    }
}

pub struct GitHubClient {
    octocrab: Arc<Octocrab>,
    organization: String,
    cache: Arc<RwLock<ClientCache>>,
}

struct ClientCache {
    repositories: Option<(Vec<Repository>, DateTime<Utc>)>,
    cache_duration: std::time::Duration,
}

impl GitHubClient {
    pub async fn new(token: Option<String>, organization: String) -> Result<Self> {
        let mut builder = Octocrab::builder();
        
        if let Some(token) = token {
            builder = builder.personal_token(token);
        }
        
        let octocrab = builder
            .build()
            .map_err(|e| Error::GitHub(e))?;
        
        Ok(Self {
            octocrab: Arc::new(octocrab),
            organization,
            cache: Arc::new(RwLock::new(ClientCache {
                repositories: None,
                cache_duration: std::time::Duration::from_secs(300), // 5 minutes
            })),
        })
    }
    
    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((repos, cached_at)) = &cache.repositories {
                if cached_at.signed_duration_since(Utc::now()).num_seconds().abs() < cache.cache_duration.as_secs() as i64 {
                    return Ok(repos.clone());
                }
            }
        }
        
        // Fetch from API
        let mut all_repos = Vec::new();
        let mut page = 1u32;
        
        loop {
            let repos: Page<octocrab::models::Repository> = self
                .octocrab
                .orgs(&self.organization)
                .list_repos()
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| Error::GitHub(e))?;
            
            let has_next = repos.next.is_some();
            
            for repo in repos.items {
                all_repos.push(Repository {
                    id: repo.id.0,
                    name: repo.name,
                    full_name: repo.full_name,
                    description: repo.description,
                    private: repo.private.unwrap_or(false),
                    fork: repo.fork.unwrap_or(false),
                    created_at: repo.created_at.unwrap_or_else(Utc::now),
                    updated_at: repo.updated_at.unwrap_or_else(Utc::now),
                    pushed_at: repo.pushed_at,
                    homepage: repo.homepage,
                    size: repo.size.unwrap_or(0) as u64,
                    stars: repo.stargazers_count.unwrap_or(0),
                    watchers: repo.watchers_count.unwrap_or(0),
                    forks: repo.forks_count.unwrap_or(0),
                    open_issues_count: repo.open_issues_count.unwrap_or(0),
                    language: repo.language,
                    has_issues: repo.has_issues.unwrap_or(true),
                    has_projects: repo.has_projects.unwrap_or(true),
                    has_wiki: repo.has_wiki.unwrap_or(true),
                    has_pages: repo.has_pages.unwrap_or(false),
                    has_downloads: repo.has_downloads.unwrap_or(true),
                    archived: repo.archived.unwrap_or(false),
                    disabled: repo.disabled.unwrap_or(false),
                    license: repo.license.map(|l| License {
                        key: l.key,
                        name: l.name,
                        spdx_id: l.spdx_id,
                        url: l.url,
                    }),
                    default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
                    url: repo.html_url.to_string(),
                    clone_url: repo.clone_url.to_string(),
                });
            }
            
            if !has_next {
                break;
            }
            
            page += 1;
        }
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.repositories = Some((all_repos.clone(), Utc::now()));
        }
        
        Ok(all_repos)
    }
    
    pub async fn get_repository(&self, name: &str) -> Result<Repository> {
        let repo = self
            .octocrab
            .repos(&self.organization, name)
            .get()
            .await
            .map_err(|e| Error::GitHub(e))?;
        
        Ok(Repository {
            id: repo.id.0,
            name: repo.name,
            full_name: repo.full_name,
            description: repo.description,
            private: repo.private.unwrap_or(false),
            fork: repo.fork.unwrap_or(false),
            created_at: repo.created_at.unwrap_or_else(Utc::now),
            updated_at: repo.updated_at.unwrap_or_else(Utc::now),
            pushed_at: repo.pushed_at,
            homepage: repo.homepage,
            size: repo.size.unwrap_or(0) as u64,
            stars: repo.stargazers_count.unwrap_or(0),
            watchers: repo.watchers_count.unwrap_or(0),
            forks: repo.forks_count.unwrap_or(0),
            open_issues_count: repo.open_issues_count.unwrap_or(0),
            language: repo.language,
            has_issues: repo.has_issues.unwrap_or(true),
            has_projects: repo.has_projects.unwrap_or(true),
            has_wiki: repo.has_wiki.unwrap_or(true),
            has_pages: repo.has_pages.unwrap_or(false),
            has_downloads: repo.has_downloads.unwrap_or(true),
            archived: repo.archived.unwrap_or(false),
            disabled: repo.disabled.unwrap_or(false),
            license: repo.license.map(|l| License {
                key: l.key,
                name: l.name,
                spdx_id: l.spdx_id,
                url: l.url,
            }),
            default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
            url: repo.html_url.to_string(),
            clone_url: repo.clone_url.to_string(),
        })
    }
    
    pub async fn list_issues(&self, repo_name: &str) -> Result<Vec<Issue>> {
        let issues = self
            .octocrab
            .issues(&self.organization, repo_name)
            .list()
            .per_page(100)
            .send()
            .await
            .map_err(|e| Error::GitHub(e))?;
        
        Ok(issues
            .items
            .into_iter()
            .map(|issue| Issue {
                id: issue.id.0,
                number: issue.number,
                title: issue.title,
                body: issue.body,
                state: issue.state.to_string(),
                created_at: issue.created_at,
                updated_at: issue.updated_at,
                closed_at: issue.closed_at,
                labels: issue
                    .labels
                    .into_iter()
                    .map(|l| Label {
                        name: l.name,
                        color: l.color,
                        description: l.description,
                    })
                    .collect(),
                assignees: issue
                    .assignees
                    .into_iter()
                    .map(|a| a.login)
                    .collect(),
            })
            .collect())
    }
    
    pub async fn list_pull_requests(&self, repo_name: &str) -> Result<Vec<PullRequest>> {
        let prs = self
            .octocrab
            .pulls(&self.organization, repo_name)
            .list()
            .per_page(100)
            .send()
            .await
            .map_err(|e| Error::GitHub(e))?;
        
        Ok(prs
            .items
            .into_iter()
            .map(|pr| PullRequest {
                id: pr.id.0,
                number: pr.number,
                title: pr.title.unwrap_or_default(),
                body: pr.body,
                state: format!("{:?}", pr.state),
                created_at: pr.created_at.unwrap_or_else(Utc::now),
                updated_at: pr.updated_at.unwrap_or_else(Utc::now),
                closed_at: pr.closed_at,
                merged_at: pr.merged_at,
                draft: pr.draft.unwrap_or(false),
            })
            .collect())
    }
}