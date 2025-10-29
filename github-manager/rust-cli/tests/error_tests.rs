use llamasearch_cli::error::{Error, ErrorContext, ErrorExt};
use std::io;

#[test]
fn test_error_types() {
    let io_err = Error::Io(io::Error::new(io::ErrorKind::NotFound, "test"));
    assert!(matches!(io_err, Error::Io(_)));
    
    let auth_err = Error::Auth("Invalid token".to_string());
    assert!(auth_err.is_auth_error());
    
    let network_err = Error::Network("Connection failed".to_string());
    assert!(network_err.is_retryable());
    
    let not_found = Error::FileNotFound("test.txt".to_string());
    assert!(not_found.is_not_found());
}

#[test]
fn test_error_context() {
    let ctx = ErrorContext::new("Test error")
        .with_context("During file operation")
        .with_suggestion("Check file permissions");
    
    assert_eq!(ctx.error, "Test error");
    assert_eq!(ctx.context.unwrap(), "During file operation");
    assert_eq!(ctx.suggestion.unwrap(), "Check file permissions");
}

#[test]
fn test_error_extension() {
    fn failing_operation() -> Result<(), Error> {
        Err(Error::Unknown("operation failed".to_string()))
    }
    
    let result = failing_operation()
        .context("while processing file")
        .with_context(|| format!("at line {}", 42));
    
    assert!(result.is_err());
}