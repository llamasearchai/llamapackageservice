use crate::error::Result;
use mockito::{Server, ServerGuard, Mock};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use reqwest::Client;

pub struct TestContext {
    pub temp_dir: TempDir,
    pub mock_server: ServerGuard,
    pub output_dir: PathBuf,
}

impl TestContext {
    pub fn new() -> std::io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let mock_server = mockito::Server::new();
        let output_dir = temp_dir.path().join("output");
        std::fs::create_dir_all(&output_dir)?;

        Ok(Self {
            temp_dir,
            mock_server,
            output_dir,
        })
    }

    pub fn mock_github_repo(&mut self, owner: &str, repo: &str) -> Mock {
        self.mock_server
            .mock("GET", format!("/repos/{}/{}", owner, repo).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "name": repo,
                    "description": "Test repository",
                    "default_branch": "main",
                    "owner": {
                        "login": owner
                    }
                })
                .to_string(),
            )
            .create()
    }

    pub fn mock_github_rate_limit(&mut self) -> Mock {
        self.mock_server.mock("GET", "/rate_limit")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "resources": {
                    "core": {
                        "limit": 5000,
                        "remaining": 4999,
                        "reset": 1644982396
                    }
                }
            }).to_string())
            .create()
    }

    pub fn mock_pypi_package(&mut self, package_name: &str) -> Mock {
        self.mock_server
            .mock("GET", format!("/pypi/{}/json", package_name).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "info": {
                        "name": package_name,
                        "version": "1.0.0",
                        "description": "Test package"
                    }
                })
                .to_string(),
            )
            .create()
    }

    pub fn mock_npm_package(&mut self, package_name: &str) -> Mock {
        self.mock_server
            .mock("GET", format!("/{}", package_name).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "name": package_name,
                    "version": "1.0.0",
                    "description": "Test package"
                })
                .to_string(),
            )
            .create()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[test]
    fn test_context_creation() {
        let context = TestContext::new();
        assert!(context.is_ok());
        let context = context.unwrap();
        assert!(context.output_dir.exists());
    }

    #[tokio::test]
    async fn test_github_mocks() {
        let mut context = TestContext::new().unwrap();
        let mock = context.mock_github_repo("test-owner", "test-repo");
        let client = Client::new();
        let response = client
            .get(&format!("{}/repos/test-owner/test-repo", context.mock_server.url()))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        mock.assert();
    }

    #[tokio::test]
    async fn test_pypi_mocks() {
        let mut context = TestContext::new().unwrap();
        let mock = context.mock_pypi_package("test-package");
        let client = Client::new();
        let response = client
            .get(&format!("{}/pypi/test-package/json", context.mock_server.url()))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        mock.assert();
    }

    #[tokio::test]
    async fn test_npm_mocks() {
        let mut context = TestContext::new().unwrap();
        let mock = context.mock_npm_package("test-package");
        let client = Client::new();
        let response = client
            .get(&format!("{}/test-package", context.mock_server.url()))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        mock.assert();
    }
} 