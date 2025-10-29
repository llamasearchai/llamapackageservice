use llamapackageservice::config::Config;
use llamapackageservice::processors::github::GitHubProcessor;
use llamapackageservice::processors::PackageProcessor;
use tempfile::TempDir;

#[tokio::test]
#[ignore]
async fn test_github_processor() {
    let config = Config::default();
    let processor = GitHubProcessor::new();
    let temp_dir = TempDir::new().unwrap();
    
    let result = processor.process("https://github.com/rust-lang/rust", temp_dir.path(), &config).await;
    assert!(result.is_ok());
} 