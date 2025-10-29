use crate::graphics::{show_llama_thinking_animation, show_llama_success, show_llama_error};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use crate::processors::{github, github_org, pypi};
use tempfile::TempDir;
use std::future::Future;
use std::pin::Pin;
use std::error::Error;
use std::fs;

/// Define a type alias for our test future type using the imported Error trait.
type TestResult = Result<(), Box<dyn Error + Send + Sync>>;
type TestFuture = Pin<Box<dyn Future<Output = TestResult> + Send>>;

// Function to create a boxed test future
fn box_test<F>(future: F) -> TestFuture 
where
    F: Future<Output = TestResult> + Send + 'static,
{
    Box::pin(future)
}

pub async fn run_test_suite() -> TestResult {
    println!("\n{}", "[TESTING] Running LlamaSearch Test Suite".bold().green());
    
    let tests = vec![
        ("PyPI Package Processing", box_test(test_pypi())),
        ("GitHub Repository Processing", box_test(test_github())),
        ("GitHub Organization Processing", box_test(test_github_org())),
    ];

    let pb = ProgressBar::new(tests.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
    );

    for (name, test) in tests {
        pb.set_message(format!("Testing {}", name));
        show_llama_thinking_animation().await;
        
        match test.await {
            Ok(_) => {
                show_llama_success(&format!("[PASSED] {}", name)).await;
                pb.inc(1);
            },
            Err(e) => {
                show_llama_error(&format!("[FAILED] {}: {}", name, e)).await;
                return Err(e);
            }
        }
    }

    pb.finish_with_message("All tests completed successfully!");
    Ok(())
}

async fn test_pypi() -> TestResult {
    let temp_dir = TempDir::new()?;
    let pb = ProgressBar::new(0);
    
    pypi::process_pypi_package(
        "https://pypi.org/project/requests",
        &temp_dir.path().to_path_buf(),
        &pb,
    )
    .await?;
    
    verify_output(&temp_dir, &[
        "Package: requests",
        "Source:",
        "Version:",
        "Description:",
        "Package Contents:",
        "Dependencies:",
        "GitHub Repository Contents:"
    ])?;
    
    let output_files = fs::read_dir(temp_dir.path())?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    
    assert!(!output_files.is_empty(), "No output files generated");
    
    let content = fs::read_to_string(&output_files[0].path())?;
    assert!(content.contains("Package Contents:"), "Missing package contents section");
    assert!(content.contains("Dependencies:"), "Missing dependencies section");
    
    Ok(())
}

async fn test_github() -> TestResult {
    let temp_dir = TempDir::new()?;
    let pb = ProgressBar::new(0);
    
    github::process_github_repo(
        "https://github.com/openai/openai-cookbook",
        &temp_dir.path().to_path_buf(),
        &pb,
    )
    .await?;
    
    verify_output(&temp_dir, &["Repository: openai-cookbook", "Source:", "Processed:"])?;
    Ok(())
}

async fn test_github_org() -> TestResult {
    let temp_dir = TempDir::new()?;
    let pb = ProgressBar::new(0);
    
    github_org::process_github_org(
        "https://github.com/rust-lang",
        &temp_dir.path().to_path_buf(),
        &pb,
    )
    .await?;
    
    verify_output(
        &temp_dir,
        &[
            "GitHub Organization: rust-lang",
            "Public Repositories:",
            "Primary Language:",
            "Stars:",
        ],
    )?;
    Ok(())
}

fn verify_output(temp_dir: &TempDir, expected_contents: &[&str]) -> TestResult {
    let files: Vec<_> = std::fs::read_dir(temp_dir.path())?
        .filter_map(|entry| entry.ok())
        .collect();
        
    if files.is_empty() {
        return Err("No output file generated".into());
    }
    
    let content = std::fs::read_to_string(files[0].path())?;
    for expected in expected_contents {
        if !content.contains(expected) {
            return Err(format!("Output file missing expected content: {}", expected).into());
        }
    }
    
    Ok(())
} 