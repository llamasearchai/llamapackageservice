use crate::error::{ProcessorError, Result};
use crate::processors::common::{self, save_output_file};
use crate::processors::pypi::process_pypi_url as process_pypi_package;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header;
use scraper::{Html, Selector};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn, error};
use serde_json::Value;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

/// Processes a PyPI user profile, gathering information about their packages
///
/// This function downloads and analyzes a PyPI user profile, extracting
/// information about their packages, download statistics, and contributions.
/// It generates a comprehensive report saved to the specified output directory.
///
/// # Arguments
/// * `username` - The PyPI username to process
/// * `output_dir` - Directory where output files should be saved
/// * `pb` - Progress bar for displaying status
///
/// # Returns
/// Result indicating success or failure of the processing operation
pub async fn process_pypi_profile(
    username: &str,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    // Log start of processing
    info!("Starting to process PyPI profile for: {}", username);
    
    // Create a directory for this user
    let user_dir = output_dir.join(format!("pypi_user_{}", username));
    fs::create_dir_all(&user_dir).await?;
    info!("Created user directory: {}", user_dir.display());
    
    // Configure HTTP client with appropriate headers
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    
    // Try multiple approaches to find packages for this user
    let mut package_urls = Vec::new();
    
    // Approach 1: Try to get projects directly from PyPI API
    info!("Attempting to use PyPI API to find projects for user: {}", username);
    
    // Try the PyPI JSON API to get projects
    let api_url = format!("https://pypi.org/pypi/{}/json", username);
    let api_response = client.get(&api_url)
        .header(header::ACCEPT, "application/json")
        .send()
        .await;
    
    // If the API call succeeds, try parsing it as the user's own package
    if let Ok(response) = api_response {
        if response.status().is_success() {
            if let Ok(json) = response.json::<Value>().await {
                if let Some(info) = json.get("info") {
                    if let Some(project_name) = info.get("name").and_then(|n| n.as_str()) {
                        let project_url = format!("https://pypi.org/project/{}/", project_name);
                        info!("Found user's own package: {}", project_url);
                        package_urls.push(project_url);
                    }
                }
            }
        } else {
            info!("User {} is not a package name, status: {}", username, response.status());
        }
    }
    
    // Approach 2: Try searching PyPI for the username as an author
    info!("Searching PyPI for packages by author: {}", username);
    let search_url = format!("https://pypi.org/search/?q=author%3A{}&o=", username);
    
    // Use a regular GET request to the search page
    let search_response = client.get(&search_url)
        .header(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .send()
        .await?;
    
    if search_response.status().is_success() {
        let search_html = search_response.text().await?;
        
        // Save the search HTML for debugging
        let search_html_path = user_dir.join("search_results.html");
        fs::write(&search_html_path, &search_html).await?;
        info!("Saved search results HTML to: {}", search_html_path.display());
        
        // Parse the HTML and extract package URLs
        let document = Html::parse_document(&search_html);
        let search_results = extract_package_urls_from_search(&document);
        
        if !search_results.is_empty() {
            info!("Found {} packages from search by author", search_results.len());
            package_urls.extend(search_results);
        }
    } else {
        warn!("Failed to search PyPI for author: {}, status: {}", username, search_response.status());
    }
    
    // If we still don't have any packages, try using XMLRPC API (legacy, but might work)
    if package_urls.is_empty() {
        info!("No packages found yet. Trying to find user's projects from other sources...");
        
        // Create a brief delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Try fetching recent packages by this author from PyPI's simple index
        let simple_index_url = format!("https://pypi.org/simple/");
        let simple_response = client.get(&simple_index_url)
            .send()
            .await;
            
        if let Ok(response) = simple_response {
            if response.status().is_success() {
                let index_html = response.text().await?;
                
                // Save a sample of the index for debugging
                let sample = if index_html.len() > 10000 {
                    &index_html[0..10000]
                } else {
                    &index_html
                };
                let index_path = user_dir.join("simple_index_sample.html");
                fs::write(&index_path, sample).await?;
                
                // Parse the HTML and look for projects that might be by our user
                // This is a heuristic approach, as the simple index doesn't list authors
                if index_html.contains(&username.to_lowercase()) {
                    let document = Html::parse_document(&index_html);
                    if let Ok(selector) = Selector::parse("a") {
                        for element in document.select(&selector) {
                            if let Some(package_name) = element.text().next() {
                                if package_name.to_lowercase().contains(&username.to_lowercase()) {
                                    let package_url = format!("https://pypi.org/project/{}/", package_name);
                                    info!("Found potential package by naming convention: {}", package_url);
                                    package_urls.push(package_url);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate the URLs
    package_urls.sort();
    package_urls.dedup();
    
    // If we still have no packages, try direct package name
    if package_urls.is_empty() {
        let direct_url = format!("https://pypi.org/project/{}/", username);
        info!("No packages found for user. Trying direct package name: {}", direct_url);
        package_urls.push(direct_url);
    }
    
    if package_urls.is_empty() {
        error!("No packages found for user: {}", username);
        return Err(ProcessorError::Processing(format!("No packages found for user: {}", username)));
    }
    
    info!("Found {} packages for user {}", package_urls.len(), username);
    
    // Create a summary file
    let mut summary = String::new();
    summary.push_str(&format!("PyPI Profile: {}\n", username));
    summary.push_str(&format!("Source: https://pypi.org/user/{}/\n", username));
    summary.push_str(&format!("Total Packages: {}\n\n", package_urls.len()));
    summary.push_str("Packages:\n");
    
    for url in &package_urls {
        summary.push_str(&format!("- {}\n", url));
    }
    
    // Save summary
    let summary_path = user_dir.join(format!("profile_{}_summary.txt", username));
    fs::write(&summary_path, summary).await?;
    info!("Saved profile summary to: {}", summary_path.display());
    
    // Process each package
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, package_url) in package_urls.iter().enumerate() {
        pb.set_message(format!("Processing package {}/{}", i + 1, package_urls.len()));
        
        // Create a separate progress bar for this package
        let package_pb = indicatif::ProgressBar::new_spinner();
        package_pb.set_message(format!("Processing {}", package_url));
        
        // Process the package
        match process_pypi_package(package_url, &user_dir, &package_pb).await {
            Ok(_) => {
                info!("Successfully processed package: {}", package_url);
                package_pb.finish_with_message(format!("Completed {}", package_url));
                success_count += 1;
            }
            Err(e) => {
                error!("Failed to process package {}: {}", package_url, e);
                package_pb.finish_with_message(format!("Failed to process {}: {}", package_url, e));
                error_count += 1;
            }
        }
    }
    
    pb.finish_with_message(format!("Completed processing profile for {} ({} succeeded, {} failed)", 
        username, success_count, error_count));
    
    Ok(())
}

fn extract_package_urls_from_search(document: &Html) -> Vec<String> {
    // Define selectors for search results
    let selectors = [
        ".package-snippet a[href^='/project/']",
        "a.package-snippet",
        ".search-results a[href^='/project/']",
        "ul.unstyled li a[href^='/project/']",
    ];
    
    let mut results = Vec::new();
    
    // Try each selector
    for selector_str in selectors.iter() {
        if let Ok(selector) = Selector::parse(selector_str) {
            let links: Vec<String> = document.select(&selector)
                .filter_map(|element| {
                    element.value().attr("href").map(|href| {
                        if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("https://pypi.org{}", href)
                        }
                    })
                })
                .collect();
                
            info!("Found {} packages with search selector '{}'", links.len(), selector_str);
            
            if !links.is_empty() {
                results.extend(links);
                break; // Found packages with this selector, no need to try others
            }
        }
    }
    
    // If no packages found with specific selectors, try a more general approach
    if results.is_empty() {
        warn!("No packages found with specific selectors, trying general approach");
        
        // Try to find any links that contain /project/
        if let Ok(general_selector) = Selector::parse("a[href]") {
            results = document.select(&general_selector)
                .filter_map(|element| {
                    let href = element.value().attr("href")?;
                    if href.contains("/project/") {
                        Some(if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("https://pypi.org{}", href)
                        })
                    } else {
                        None
                    }
                })
                .collect();
                
            info!("Found {} packages with general search approach", results.len());
        }
    }
    
    // Remove duplicates
    results.sort();
    results.dedup();
    
    info!("Returning {} unique package URLs from search", results.len());
    results
}

// Keep this function for backward compatibility
fn extract_package_info(html: &str, url: &str, custom_name: &str) -> String {
    let document = Html::parse_document(html);
    
    let mut content = String::new();
    content.push_str(&format!("PyPI Package: {}\n", custom_name));
    content.push_str(&format!("Source: {}\n", url));
    content.push_str("Profile Package: true\n");

    // Extract more package information
    let selectors = [
        (".package-description__summary", "Description"),
        (".package-header__version", "Version"),
        (".package-header__name", "Name"),
        (".package-header__license", "License"),
        (".github-repo-info", "GitHub"),
        (".package-description__maintainers", "Maintainers"),
    ];

    for (selector_str, label) in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    content.push_str(&format!("{}: {}\n", label, text));
                }
            }
        }
    }
    
    content
}

fn process_profile() -> Result<()> {
    // Implementation
    Ok(())
} 