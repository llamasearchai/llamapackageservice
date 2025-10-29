#[cfg(test)]
pub mod mocks {
    use mockall::automock;
    
    #[automock]
    pub trait GitHubApi {
        async fn get_repo(&self, owner: &str, repo: &str) -> Result<Value, ProcessorError>;
        async fn get_org_repos(&self, org: &str) -> Result<Vec<Value>, ProcessorError>;
    }
} 