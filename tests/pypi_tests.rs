use llamapackageservice::processors::pypi::PyPiProcessor;
use llamapackageservice::processors::PackageProcessor;
use mockito::Matcher;
use tempfile::TempDir;
use std::path::Path;

mod common;
use common::test_helpers::*;

#[tokio::test]
async fn test_pypi_package_success() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    let _m = server.mock("GET", "/pypi/requests/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(read_test_json("pypi_package.json"))
        .create();

    let processor = PyPiProcessor::new();
    let result = processor.process("requests", temp_dir.path(), &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pypi_package_not_found() {
    setup_test_logger();
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    
    let mut server = setup_test_server().await;
    let _m = server.mock("GET", "/pypi/nonexistent-package/json")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Package not found"}"#)
        .create();

    let processor = PyPiProcessor::new();
    let result = processor.process("nonexistent-package", temp_dir.path(), &config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pypi_url_extraction() {
    let processor = PyPiProcessor::new();
    
    // Test with direct PyPI URL
    let _config = create_test_config();
    let result = processor.validate("https://pypi.org/project/requests/").await;
    assert!(result.is_ok());
    
    // Test with pip install command
    let result = processor.validate("pip install numpy").await;
    assert!(result.is_ok());
    
    // Test with just package name
    let result = processor.validate("pandas").await;
    assert!(result.is_ok());
    
    // Test with invalid format
    let result = processor.validate("https://invalid.org/something").await;
    assert!(result.is_err());
}