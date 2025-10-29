use llamapackageservice::{
    Config,
    error::{ProcessorError, Result},
    processors::{self, github, pypi, npm, crates, ProcessorFactory, common},
    parallel::ParallelProcessor,
    cache::{StringCache, Cache},
    output_organizer::{self, list_output_files, organize_output, generate_index},
};
use std::path::{PathBuf, Path};
use log::{info, error};
use colored::*;
use clap::{Parser, Subcommand, Args};
use tokio;
use yansi::Paint;
use std::process;
use indicatif::{ProgressBar, ProgressStyle};
use chrono;
use std::time::Duration;
use tokio::time::sleep;
use std::io::{self, Write};
use tokio::process::Command as TokioCommand;
use std::process::Stdio;

mod llama_ui;
use llama_ui::LlamaUI;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to process directly (skip interactive mode)
    #[arg(short, long)]
    url: Option<String>,
    
    /// Output directory
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Run in legacy interactive mode with UI
    #[arg(short, long)]
    legacy_ui: bool,
    
    /// Generate index file after processing
    #[arg(short = 'x', long)]
    index: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize memory limits
    llamapackageservice::limit_memory_usage();
    
    // Attempt to launch the web server in the background (if not already running)
    if std::env::var("LLAMA_DISABLE_SERVER").is_err() {
        if let Err(e) = spawn_server_background() {
            eprintln!("[WARNING] Unable to launch embedded web server: {}", e);
        }
    }
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Create default configuration
    let output_dir = cli.output.unwrap_or_else(|| PathBuf::from("./output"));
    let config = Config::new(output_dir.clone());
    
    // Create output directory if it doesn't exist
    tokio::fs::create_dir_all(&config.output_dir).await?;
    
    // Process according to mode
    if let Some(url) = cli.url {
        // Process URL directly if provided
        process_url(&url, &config).await?;
    } else if cli.legacy_ui {
        // Use the legacy UI if explicitly requested
        let ui = LlamaUI::new();
        ui.run(&config).await?;
    } else {
        // Use the new simple interactive mode
        run_simple_interactive(&config).await?;
    }
    
    // Organize output files into appropriate directories
    if let Err(e) = output_organizer::organize_output(&output_dir) {
        println!("Warning: Failed to organize output files: {}", e);
    }
    
    // Generate index if requested
    if cli.index {
        if let Err(e) = output_organizer::generate_index(&output_dir) {
            println!("Failed to generate index: {}", e);
        }
    }
    
    // Display the output files
    output_organizer::list_output_files(&output_dir);
    
    Ok(())
}

/// Run a simple interactive command-line interface
async fn run_simple_interactive(config: &Config) -> Result<()> {
    println!("\n{}", "Welcome to The Llama Package Service".bright_green().bold());
    println!("{}", "A tool for analyzing and documenting software packages".bright_blue());
    println!("{}\n", "================================================".bright_yellow());
    
    // Show examples of supported URL types
    println!("{}", "Examples of supported inputs:".bright_yellow());
    println!("  {} ./my-project", "Local Directory:".bright_white().bold());
    println!("  {} /path/to/my/code", "Local Directory:".bright_white().bold());
    println!("  {} ./README.md", "Local File:".bright_white().bold());
    println!("  {} https://github.com/username/repository", "GitHub:".bright_white().bold());
    println!("  {} https://pypi.org/project/package-name", "PyPI:".bright_white().bold());
    println!("  {} https://www.npmjs.com/package/package-name", "NPM:".bright_white().bold());
    println!("  {} https://crates.io/crates/crate-name", "Rust:".bright_white().bold());
    println!("  {} https://pkg.go.dev/github.com/username/package", "Go:".bright_white().bold());
    println!("  {}", "You can also use commands like 'pip install package-name'".bright_white().bold());
    
    loop {
        println!("\n{}", "Enter a URL or local path to process (or 'q' to quit, 'h' for help):".bright_cyan());
        print!("{} ", ">".bright_green());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Handle special commands
        match input.to_lowercase().as_str() {
            "q" | "quit" | "exit" => {
                println!("{}", "Thank you for using The Llama Package Service!".bright_green());
                break;
            },
            "h" | "help" | "?" => {
                print_help();
                continue;
            },
            "examples" => {
                print_examples();
                continue;
            },
            _ => {}
        }
        
        // Try to process the URL
        match process_url(input, config).await {
            Ok(_) => {
                println!("\n{}", "Processing completed successfully.".bright_green());
                println!("{}", "Enter another URL or 'q' to quit.".bright_cyan());
            },
            Err(e) => {
                println!("\n{} {}", "[ERROR]".bright_red(), format!("Error: {}", e).bright_red());
                println!("{}", "Try a different URL or enter 'examples' to see supported URL formats.".bright_cyan());
            }
        }
    }
    
    Ok(())
}

/// Print help information
fn print_help() {
    println!("\n{}", "Commands:".bright_yellow());
    println!("  {:<10} - {}", "q".bright_green(), "Quit the application");
    println!("  {:<10} - {}", "h".bright_green(), "Show this help");
    println!("  {:<10} - {}", "examples".bright_green(), "Show example URLs");
    println!("\n{}", "Simply paste or type any URL or local path from:".bright_yellow());
    println!("  - Local files and directories");
    println!("  - GitHub repositories");
    println!("  - PyPI packages");
    println!("  - NPM packages");
    println!("  - Rust crates");
    println!("  - Go packages");
}

/// Print example URLs
fn print_examples() {
    println!("\n{}", "Example inputs:".bright_yellow());
    
    println!("  {}", "Local Files/Directories:".bright_white().bold());
    println!("    ./my-project");
    println!("    /Users/username/code/my-app");
    println!("    ../other-project");
    println!("    ./src/main.rs");
    println!("    C:\\Users\\username\\Documents\\code");
    
    println!("\n  {}", "GitHub:".bright_white().bold());
    println!("    https://github.com/rust-lang/rust");
    println!("    https://github.com/microsoft/typescript");
    
    println!("\n  {}", "PyPI:".bright_white().bold());
    println!("    https://pypi.org/project/requests");
    println!("    pip install numpy");
    
    println!("\n  {}", "NPM:".bright_white().bold());
    println!("    https://www.npmjs.com/package/react");
    println!("    npm install express");
    
    println!("\n  {}", "Rust:".bright_white().bold());
    println!("    https://crates.io/crates/tokio");
    println!("    https://docs.rs/serde");
    
    println!("\n  {}", "Go:".bright_white().bold());
    println!("    https://pkg.go.dev/github.com/gofiber/fiber");
    println!("    https://pkg.go.dev/net/http");
}

/// Process a URL directly (non-interactive mode)
async fn process_url(url: &str, config: &Config) -> Result<()> {
    // Normalize the input first to handle trailing spaces and quoted paths
    let normalized = llamapackageservice::utils::normalize_url_or_path(url);
    let url_type = processors::ProcessorFactory::detect_url_type(&normalized);
    println!("{} {} ({})", "Processing URL:".bright_green(), normalized.bright_white(), url_type.bright_cyan());
    
    let pb = ProgressBar::new_spinner();
    processors::common::setup_progress_style(&pb);
    pb.set_message(format!("Processing {}", &normalized));
    
    let result = match processors::ProcessorFactory::create_processor(&normalized) {
        Ok(processor) => {
            processor.process(&normalized, &config.output_dir, config).await
        },
        Err(e) => Err(e)
    };
    
    match &result {
        Ok(_) => {
            println!("{} {}", "[SUCCESS]".bright_green(), "Processing completed successfully".bright_white());
            println!("{} {}", "[SAVED]".bright_blue(), format!("Results saved to {}", config.output_dir.display()).bright_white());
        },
        Err(e) => {
            println!("{} {}", "[ERROR]".bright_red(), format!("Error: {}", e).bright_red());
        }
    }
    
    result
}

// Helper function to create a progress bar
fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_strings(&["-", "\\", "|", "/", "-", "\\", "|", "/"])
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb
}

// The following functions are kept for reference but not actively used in the main flow

async fn process_single_url(url: &str, config: &Config) -> Result<()> {
    println!("{} {}", Paint::green("Processing:"), Paint::blue(url));
    
    let pb = create_progress_bar();
    
    let processor = ProcessorFactory::create_processor(url)?;
    processor.validate(url).await?;
    processor.process(url, &config.output_dir, config).await?;
    
    pb.finish_with_message(format!("{} {}", Paint::green("[OK]"), Paint::blue("Processing completed successfully")));
    
    println!("\n{} Results saved to {}", 
        Paint::green("[OK]"),
        Paint::blue(config.output_dir.display())
    );
    
    Ok(())
}

async fn process_batch(input_file: &str, config: &Config) -> Result<()> {
    let content = tokio::fs::read_to_string(input_file).await?;
    let urls: Vec<_> = content.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    
    println!("{} {} URLs from {}", 
        Paint::green("Processing"),
        Paint::blue(urls.len()),
        Paint::blue(input_file)
    );
    
    // Default to 5 if not specified in config
    let parallelism = 5;
    let parallel = ParallelProcessor::new(parallelism);
    
    let tasks = urls.iter()
        .map(|url| {
            let url = url.to_string();
            let config = config.clone();
            async move {
                let processor = ProcessorFactory::create_processor(&url)?;
                processor.process(&url, &config.output_dir, &config).await
            }
        })
        .collect();
    
    let results = parallel.process(tasks).await;
    
    // Report results
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let error_count = results.len() - success_count;
    
    println!("\n{} {} URLs processed successfully, {} failed", 
        Paint::green("[OK]"),
        Paint::blue(success_count),
        Paint::red(error_count)
    );
    
    // Organize output files
    if let Err(e) = output_organizer::organize_output(&config.output_dir) {
        println!("Warning: Failed to organize output files: {}", e);
    }
    
    // Generate index
    if let Err(e) = output_organizer::generate_index(&config.output_dir) {
        println!("Warning: Failed to generate index: {}", e);
    }
    
    Ok(())
}

async fn analyze_repo(repo_url: &str, config: &Config) -> Result<()> {
    println!("{} {}", Paint::green("Analyzing repository:"), Paint::blue(repo_url));
    
    // First download the repository
    let pb = create_progress_bar();
    pb.set_message("Downloading repository...");
    
    // Download repo content
    let content = github::download_repository(repo_url).await?;
    pb.set_message("Repository downloaded");
    
    // Save raw content
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let repo_name = extract_repo_name(repo_url);
    let filename = format!("{}_{}_github_repo.txt", timestamp, repo_name);
    let output_path = config.output_dir.join(&filename);
    tokio::fs::write(&output_path, &content).await?;
    
    // Process repo content
    pb.set_message("Processing repository content...");
    let processed_content = github::process_repository_content(&content)?;
    
    // Save processed content
    let processed_filename = format!("{}_{}_github_repo_processed.txt", timestamp, repo_name);
    let processed_path = config.output_dir.join(&processed_filename);
    tokio::fs::write(&processed_path, &processed_content).await?;
    
    pb.finish_with_message(format!("{} {}", Paint::green("[OK]"), Paint::blue("Repository processed successfully")));
    
    println!("\n{} Results saved to:", Paint::green("[OK]"));
    println!("  - Raw content: {}", Paint::blue(output_path.display()));
    println!("  - Processed content: {}", Paint::blue(processed_path.display()));
    
    // Organize output files
    if let Err(e) = output_organizer::organize_output(&config.output_dir) {
        println!("Warning: Failed to organize output files: {}", e);
    }
    
    Ok(())
}

// Helper function to extract repository name from URL
fn extract_repo_name(repo_url: &str) -> String {
    // Handle GitHub repository URLs
    // https://github.com/username/repository
    if repo_url.contains("github.com") {
        let parts: Vec<&str> = repo_url.split('/').collect();
        if parts.len() >= 5 {
            return parts[4].trim_end_matches(".git").to_string();
        }
    }
    
    // Handle GitLab repository URLs
    // https://gitlab.com/username/repository
    if repo_url.contains("gitlab.com") {
        let parts: Vec<&str> = repo_url.split('/').collect();
        if parts.len() >= 5 {
            return parts[4].trim_end_matches(".git").to_string();
        }
    }
    
    // Fallback to timestamp
    chrono::Utc::now().timestamp().to_string()
}

async fn demo_cache() {
    println!("\n=== Cache Demo ===");
    let cache: Cache<String> = Cache::new(Duration::from_secs(5));
    
    // Set some values
    cache.set("key1", "value1".to_string()).await;
    cache.set("key2", "value2".to_string()).await;
    
    println!("Cache size: {}", cache.len().await);
    println!("key1 value: {:?}", cache.get("key1").await);
    
    // Wait for expiration
    println!("Waiting for 6 seconds to test expiration...");
    sleep(Duration::from_secs(6)).await;
    
    println!("key1 after expiration: {:?}", cache.get("key1").await);
    
    // Cleanup
    let removed = cache.cleanup_expired().await;
    println!("Removed {} expired entries", removed);
    println!("Cache size after cleanup: {}", cache.len().await);
}

async fn demo_string_cache() -> Result<()> {
    println!("\n=== String Cache Demo ===");
    let cache_dir = Path::new("./cache");
    let mut cache = StringCache::new(cache_dir).await?;
    
    // Set a shorter TTL for the demo
    cache.set_ttl(Duration::from_secs(5));
    
    // Set some values
    cache.set("persistent1", "This value will be saved").await?;
    cache.set("persistent2", "This value will also be saved").await?;
    
    println!("Cache size: {}", cache.len().await);
    println!("persistent1 value: {:?}", cache.get("persistent1").await?);
    
    // Save to disk
    cache.save().await?;
    println!("Cache saved to disk");
    
    // Create a new cache instance to simulate program restart
    println!("Creating new cache instance (simulating restart)");
    let new_cache = StringCache::new(cache_dir).await?;
    
    // Load from disk
    println!("Loading cache from disk");
    println!("New cache size after load: {}", new_cache.len().await);
    println!("persistent1 value from new cache: {:?}", new_cache.get("persistent1").await?);
    
    // Wait for expiration
    println!("Waiting for 6 seconds to test expiration...");
    sleep(Duration::from_secs(6)).await;
    
    // Cleanup
    let removed = new_cache.cleanup_expired().await;
    println!("Removed {} expired entries", removed);
    println!("Cache size after cleanup: {}", new_cache.len().await);
    
    Ok(())
}

/// Spawn the `server` binary (Axum REST API) in the background so the user can access it while
/// interacting with the CLI. If the binary cannot be found or fails to start, an error is
/// returned. When the web-server port (8000) is already in use the child exits immediately – we
/// ignore that fact and continue running the CLI.
fn spawn_server_background() -> std::io::Result<()> {
    // Determine path to the compiled `server` binary next to the current executable
    let current_exe = std::env::current_exe()?;
    let server_exe = current_exe.with_file_name("server");

    // Spawn detached child; stdout/stderr inherited so logs appear in the same terminal
    TokioCommand::new(server_exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}