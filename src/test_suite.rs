use std::path::PathBuf;
use crate::error::ProcessorError;
use crate::graphics::{show_llama_thinking_animation, show_llama_success, show_llama_error};
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;
use reqwest::Client;
use which::which;
use std::future::Future;
use std::pin::Pin;
use crate::output_organizer::OutputPaths;
use tempfile::TempDir;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct TestSuite {
    multi_progress: MultiProgress,
    total_progress: ProgressBar,
    tests: Vec<TestCase>,
}

struct TestCase {
    name: String,
    test_fn: Box<dyn Fn() -> BoxFuture<'static, Result<(), ProcessorError>> + Send + Sync>,
}

impl TestSuite {
    pub fn new() -> Self {
        let multi_progress = MultiProgress::new();
        let total_progress = multi_progress.add(ProgressBar::new(0));
        total_progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=> ")
        );

        TestSuite {
            multi_progress,
            total_progress,
            tests: Vec::new(),
        }
    }

    pub fn add_test<F, Fut>(&mut self, name: &str, test_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ProcessorError>> + Send + 'static,
    {
        self.tests.push(TestCase {
            name: name.to_string(),
            test_fn: Box::new(move || Box::pin(test_fn())),
        });
        self.total_progress.set_length(self.tests.len() as u64);
    }

    pub async fn run(&self) -> Result<(), ProcessorError> {
        println!("\n{}", "[TESTING] Running Installation Tests".bold().green());
        
        for test in &self.tests {
            let pb = self.multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message(format!("Testing: {}", test.name));

            show_llama_thinking_animation().await;

            match (test.test_fn)().await {
                Ok(_) => {
                    show_llama_success(&format!("✅ {} passed", test.name)).await;
                    self.total_progress.inc(1);
                },
                Err(e) => {
                    show_llama_error(&format!("❌ {} failed: {}", test.name, e)).await;
                    return Err(e);
                }
            }
            pb.finish_and_clear();
        }

        self.total_progress.finish_with_message("All installation tests completed successfully! 🎉");
        Ok(())
    }
}

pub async fn run_installation_tests() -> Result<(), ProcessorError> {
    println!("\n🧪 Running Installation Tests");
    let pb = ProgressBar::new(4);
    pb.set_message("Running tests...");

    // Test 1: Output Directory Creation
    let temp_dir = TempDir::new().map_err(|e| ProcessorError::IO(e))?;
    let output_paths = OutputPaths::new(temp_dir.path());
    output_paths.ensure_dirs_exist()?;
    pb.inc(1);

    // Test 2: GitHub API Connection
    test_github_connection().await?;
    pb.inc(1);

    // Test 3: File System Permissions
    test_file_permissions(&temp_dir.path().to_path_buf())?;
    pb.inc(1);

    // Test 4: Network Connectivity
    test_network_connectivity().await?;
    pb.inc(1);

    pb.finish_with_message("All installation tests passed successfully! 🎉");
    Ok(())
}

async fn test_github_connection() -> Result<(), ProcessorError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/rate_limit")
        .header("User-Agent", "LlamaSearch")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ProcessorError::Network("GitHub API test failed".into()));
    }
    Ok(())
}

fn test_file_permissions(test_dir: &PathBuf) -> Result<(), ProcessorError> {
    // Test write permissions
    let test_file = test_dir.join("test_permissions.txt");
    std::fs::write(&test_file, "test content")
        .map_err(|e| ProcessorError::IO(e))?;
    std::fs::remove_file(test_file)
        .map_err(|e| ProcessorError::IO(e))?;
    Ok(())
}

async fn test_network_connectivity() -> Result<(), ProcessorError> {
    let client = reqwest::Client::new();
    client
        .get("https://api.github.com")
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_installation_checks() {
        assert!(run_installation_tests().await.is_ok());
    }

    #[tokio::test]
    async fn test_github_connection() {
        assert!(super::test_github_connection().await.is_ok());
    }

    #[tokio::test]
    async fn test_file_system() {
        let temp_dir = TempDir::new().unwrap();
        assert!(super::test_file_permissions(&temp_dir.path().to_path_buf()).is_ok());
    }

    #[tokio::test]
    async fn test_network() {
        assert!(super::test_network_connectivity().await.is_ok());
    }
} 