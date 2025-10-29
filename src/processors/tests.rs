use super::*;
use crate::test_utils::TestContext;
use crate::Config;
use indicatif::ProgressBar;
use std::fs;
use std::path::PathBuf;
use std::io;
use tokio::fs::File;

#[tokio::test]
async fn test_github_processor() {
    let mut context = TestContext::new().unwrap();
    
    // Mock GitHub API endpoints
    let repo_mock = context.mock_github_repo("test-owner", "test-repo");
    let rate_limit_mock = context.mock_github_rate_limit();
    
    // Create test configuration
    let mut config = Config::default();
    config.output_dir = context.output_dir.clone();
    
    // Create progress bar for testing
    let pb = ProgressBar::hidden();
    
    // Process GitHub repository
    let result = github::process_github_repo(
        "https://github.com/test-owner/test-repo",
        &config.output_dir,
        &pb
    ).await;
    
    assert!(result.is_ok());
    
    // Verify mocks were called
    repo_mock.assert();
    rate_limit_mock.assert();
    
    // Verify output file was created
    let files: Vec<_> = fs::read_dir(&config.output_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!files.is_empty());
}

#[tokio::test]
async fn test_pypi_processor() {
    let mut context = TestContext::new().unwrap();
    
    // Mock PyPI API endpoint
    let package_mock = context.mock_pypi_package("test-package");
    
    // Create test configuration
    let mut config = Config::default();
    config.output_dir = context.output_dir.clone();
    
    // Create progress bar for testing
    let pb = ProgressBar::hidden();
    
    // Process PyPI package
    let result = pypi::process_pypi_package(
        "https://pypi.org/project/test-package",
        &config.output_dir,
        &pb
    ).await;
    
    assert!(result.is_ok());
    
    // Verify mock was called
    package_mock.assert();
    
    // Verify output file was created
    let files: Vec<_> = fs::read_dir(&config.output_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!files.is_empty());
}

#[tokio::test]
async fn test_npm_processor() {
    let mut context = TestContext::new().unwrap();
    
    // Mock NPM registry endpoint
    let package_mock = context.mock_npm_package("test-package");
    
    // Create test configuration
    let mut config = Config::default();
    config.output_dir = context.output_dir.clone();
    
    // Create progress bar for testing
    let pb = ProgressBar::hidden();
    
    // Process NPM package
    let result = npm::process_npm_package(
        "https://www.npmjs.com/package/test-package",
        &config.output_dir,
        &pb
    ).await;
    
    assert!(result.is_ok());
    
    // Verify mock was called
    package_mock.assert();
    
    // Verify output file was created
    let files: Vec<_> = fs::read_dir(&config.output_dir)
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!files.is_empty());
}

#[tokio::test]
async fn test_error_handling() {
    let mut context = TestContext::new().unwrap();
    let pb = ProgressBar::hidden();
    
    // Test invalid GitHub URL
    let result = github::process_github_repo(
        "https://github.com/invalid/repo",
        &context.output_dir,
        &pb
    ).await;
    assert!(result.is_err());
    
    // Test invalid PyPI package
    let result = pypi::process_pypi_package(
        "https://pypi.org/project/nonexistent-package",
        &context.output_dir,
        &pb
    ).await;
    assert!(result.is_err());
    
    // Test invalid NPM package
    let result = npm::process_npm_package(
        "https://www.npmjs.com/package/nonexistent-package",
        &context.output_dir,
        &pb
    ).await;
    assert!(result.is_err());
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();
    
    // Test invalid output directory
    config.output_dir = std::path::PathBuf::from("/nonexistent/directory");
    let result = config.ensure_directories_exist();
    assert!(result.is_err());
    
    // Test valid configuration
    let temp_dir = tempfile::tempdir().unwrap();
    config.output_dir = temp_dir.path().to_path_buf();
    let result = config.ensure_directories_exist();
    assert!(result.is_ok());
}

#[test]
fn test_file_exclusions() {
    let config = Config::default();
    
    // Test excluded file extensions
    let test_path = std::path::PathBuf::from("test.exe");
    assert!(config.is_excluded_file(&test_path));
    
    // Test excluded directories
    let test_path = std::path::PathBuf::from("node_modules/test.js");
    assert!(config.is_excluded_file(&test_path));
    
    // Test allowed file
    let test_path = std::path::PathBuf::from("src/main.rs");
    assert!(!config.is_excluded_file(&test_path));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processors::{
        github::GitHubProcessor,
        npm::NPMProcessor,
        pypi::PyPIProcessor,
        crates::CratesProcessor,
    };
    use tempfile::TempDir;
    use tokio::fs;
    use std::path::PathBuf;

    async fn setup_test_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir).await.unwrap();
        (temp_dir, output_dir)
    }

    #[tokio::test]
    async fn test_github_processor() -> Result<()> {
        let (_temp_dir, output_dir) = setup_test_dir().await;
        let processor = GitHubProcessor::new();
        
        // Test URL validation
        assert!(processor.validate("https://github.com/rust-lang/rust").await.is_ok());
        assert!(processor.validate("https://invalid.com/repo").await.is_err());
        
        // Test processor creation
        let url = "https://github.com/rust-lang/rust";
        let processor = create_processor(url).unwrap();
        assert!(processor.validate(url).await.is_ok());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_npm_processor() -> Result<()> {
        let (_temp_dir, output_dir) = setup_test_dir().await;
        let processor = NPMProcessor::new();
        
        // Test URL validation
        assert!(processor.validate("https://www.npmjs.com/package/express").await.is_ok());
        assert!(processor.validate("https://invalid.com/package").await.is_err());
        
        // Test processor creation
        let url = "https://www.npmjs.com/package/express";
        let processor = create_processor(url).unwrap();
        assert!(processor.validate(url).await.is_ok());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_pypi_processor() -> Result<()> {
        let (_temp_dir, output_dir) = setup_test_dir().await;
        let processor = PyPIProcessor::new();
        
        // Test URL validation
        assert!(processor.validate("https://pypi.org/project/requests").await.is_ok());
        assert!(processor.validate("https://invalid.com/project").await.is_err());
        
        // Test processor creation
        let url = "https://pypi.org/project/requests";
        let processor = create_processor(url).unwrap();
        assert!(processor.validate(url).await.is_ok());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_crates_processor() -> Result<()> {
        let (_temp_dir, output_dir) = setup_test_dir().await;
        let processor = CratesProcessor::new();
        
        // Test URL validation
        assert!(processor.validate("https://crates.io/crates/tokio").await.is_ok());
        assert!(processor.validate("https://invalid.com/crates").await.is_err());
        
        // Test processor creation
        let url = "https://crates.io/crates/tokio";
        let processor = create_processor(url).unwrap();
        assert!(processor.validate(url).await.is_ok());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_processor_clone() -> Result<()> {
        let github = GitHubProcessor::new();
        let npm = NPMProcessor::new();
        let pypi = PyPIProcessor::new();
        let crates = CratesProcessor::new();
        
        let _github_clone = github.clone();
        let _npm_clone = npm.clone();
        let _pypi_clone = pypi.clone();
        let _crates_clone = crates.clone();
        
        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<()> {
        let processor = GitHubProcessor::new();
        
        // Test invalid URL
        let result = processor.validate("invalid-url").await;
        assert!(result.is_err());
        
        // Test nonexistent repo
        let result = processor.process(
            "https://github.com/nonexistent/repo",
            &PathBuf::new(),
            &Config::default()
        ).await;
        assert!(result.is_err());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_operations() -> Result<()> {
        let (temp_dir, output_dir) = setup_test_dir().await;
        
        // Test file writing
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").await?;
        
        // Test file reading
        let content = fs::read_to_string(&test_file).await?;
        assert_eq!(content, "test content");
        
        // Test output file
        let output_file = output_dir.join("output.txt");
        fs::write(&output_file, "output content").await?;
        let content = fs::read_to_string(&output_file).await?;
        assert_eq!(content, "output content");
        
        Ok(())
    }
} 