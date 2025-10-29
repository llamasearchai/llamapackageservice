#[cfg(test)]
mod doc_tests {
    use llamapackageservice::config::Config;
    use llamapackageservice::processors::github::GitHubProcessor;
    use llamapackageservice::processors::PackageProcessor;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_readme_example() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let processor = GitHubProcessor::new();
        let temp_dir = TempDir::new()?;
        let result = processor.process("https://github.com/rust-lang/rust", temp_dir.path(), &config).await;
        assert!(result.is_ok());
        Ok(())
    }
} 