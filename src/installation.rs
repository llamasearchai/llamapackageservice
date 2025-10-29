use crate::config::Config;
use crate::error::ProcessorError;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

pub async fn run_installation_checks() -> Result<(), ProcessorError> {
    println!("\n[INSTALLATION] Running Installation Tests");
    let pb = ProgressBar::new(4);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap());

    // Check 1: Output Directory
    pb.set_message("Testing: Output Directory Creation");
    check_output_directories()?;
    pb.inc(1);

    // Check 2: GitHub API
    pb.set_message("Testing: GitHub API Connection");
    check_github_api().await?;
    pb.inc(1);

    // ... other checks ...

    pb.finish_with_message("All installation tests completed successfully! 🎉");
    Ok(())
}

async fn check_github_api() -> Result<(), ProcessorError> {
    let config = Config::load()?;
    
    if config.github_token.is_none() {
        println!("\n[WARNING] No GitHub API token found!");
        if dialoguer::Confirm::new()
            .with_prompt("Would you like to set up a GitHub API token now?")
            .interact()
            .unwrap_or(false)
        {
            let token = Config::setup_github_token()?;
            let mut config = config;
            config.github_token = Some(token);
            config.save()?;
            println!("[SUCCESS] GitHub API token saved successfully!");
        }
    }

    // Test the API connection
    let client = reqwest::Client::new();
    let mut request = client.get("https://api.github.com/rate_limit");
    
    if let Some(token) = config.github_token.as_ref() {
        request = request.header("Authorization", format!("token {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| ProcessorError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ProcessorError::Config("GitHub API test failed".into()));
    }

    Ok(())
} 