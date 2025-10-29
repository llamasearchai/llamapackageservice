use crate::config::Config;
use crate::error::{ProcessorError, Result};
use crate::processors::github::process_github_content;
use crate::processors::pypi::process_pypi_package;
use crate::processors::common::save_output_file;
use futures::TryFutureExt; // To use map_err on futures
use reqwest::Url;
use std::path::Path;
use tracing::info;
use serde_json;

pub async fn process_code_url(url: &str, output_dir: &Path) -> Result<String> {
    info!("Processing URL: {}", url);
    
    let parsed_url = Url::parse(url).map_err(ProcessorError::UrlParse)?;
    let host = parsed_url.host_str().ok_or_else(|| {
        ProcessorError::Validation("Invalid URL: no host found".into())
    })?;

    match host {
        "github.com" => {
            // Process GitHub URL
            let content = process_github_content(url, output_dir).await?;
            // Save output to "github_output.txt" within output_dir
            let out_file = output_dir.join("github_output.txt");
            save_output_file(&out_file, &content).await?;
            Ok(content)
        },
        "pypi.org" => {
            // Process PyPI URL – the path should be trimmed to get the package identifier.
            let package = process_pypi_package(&parsed_url.path().trim_matches('/'), output_dir).await?;
            let content = serde_json::to_string_pretty(&package)
                .map_err(ProcessorError::from)?;
            // Save the PyPI package info as "<package_name>.json"
            let out_file = output_dir.join(format!("{}.json", package.name));
            save_output_file(&out_file, &content).await?;
            Ok(content)
        },
        _ => Err(ProcessorError::Validation(format!("Unsupported code host: {}", host))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tracing_subscriber;

    #[tokio::test]
    async fn test_github_url() {
        let _ = tracing_subscriber::fmt::try_init();
        let temp = tempdir().unwrap();
        // A known public repository URL
        let url = "https://github.com/ultrafunkamsterdam/nodriver";
        let result = process_code_url(url, temp.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pypi_url() {
        let _ = tracing_subscriber::fmt::try_init();
        let temp = tempdir().unwrap();
        // A known PyPI package URL
        let url = "https://pypi.org/project/requests/";
        let result = process_code_url(url, temp.path()).await;
        assert!(result.is_ok());
    }
}